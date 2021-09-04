use crate::model::link::{LinkInfo, LinkMeta, LinkSpec};
use crate::model::res::{ResMeta, ResSpec};
use crate::model::rule::load_rule;
use crate::storage_backends;

use crate::error::{DataLoaderError, DataMemStorageError};
use crate::model::object::{ObjectID, ObjectIDRef};
use crate::rule_engine::Rule;
use serde_json;

use crate::model::RootCommon;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use super::{CFGCenterInner, MemStorage};

pub struct Loader {}

impl Loader {
    pub fn new() -> Self {
        return Self {};
    }

    pub fn load_by_link(
        link_path: &str,
        link_data: &[u8],
        backend: &(dyn storage_backends::StorageBackend + Send + Sync),
        mem_store: &mut MemStorage,
    ) -> Result<(), DataLoaderError> {
        let root = serde_json::from_slice::<RootCommon>(link_data)?;
        let _meta = serde_json::from_value::<LinkMeta>(root.meta)?;
        let spec = serde_json::from_value::<LinkSpec>(root.spec)?;

        if !spec.pri.is_finite() {
            return Err(DataLoaderError::SpecParseError(
                "pri field is not a valid float number".to_string(),
            ));
        }

        let mut target = LinkInfo {
            pri: spec.pri,
            is_neg: spec.is_neg,
            reses_path: Vec::new(),
            link_path: link_path.to_string(),
            rule_path: spec.rule.clone(),
        };

        // load all res that this rule depends on
        for res in spec.reses.iter() {
            if res.starts_with("path:/") && res.len() > 6 {
                let res = remove_path_type_prefix(&res);

                mem_store
                    .res_stor
                    .load_or_ref_res(res, |key| {
                        let oid = backend
                            .get_hash_by_path("master", &("/reses".to_string() + &key))
                            .map_err(|_| DataLoaderError::ObjectNotFoundError(key.to_owned()))
                            .unwrap();

                        let res_raw_data = backend.get_obj_by_hash(&oid).unwrap();
                        let root = serde_json::from_slice::<RootCommon>(&res_raw_data).unwrap();
                        let _meta = serde_json::from_value::<ResMeta>(root.meta).unwrap();
                        let spec = serde_json::from_value::<ResSpec>(root.spec).unwrap();

                        let ret: Vec<_> = spec
                            .0
                            .into_iter()
                            .map(|e| {
                                Arc::new(KeyValuePair {
                                    content_type: e.content_type,
                                    key: e.key,
                                    value: e.data,
                                })
                            })
                            .collect();
                        return Resource { data: ret, res_path: Arc::new(res.to_owned())};
                    })
                    .map_err(|_| DataLoaderError::ObjectNotFoundError(res.to_owned()))?;

                target.reses_path.push(res.to_owned());
            } else {
                return Err(DataLoaderError::ObjectNotFoundError(
                    "only support find object by path".to_string(),
                ));
            }
        }

        if spec.rule.starts_with("path:/") && spec.rule.len() > 6 {
            let rule = remove_path_type_prefix(&spec.rule);
            mem_store
                .rule_stor
                .load_or_ref_rule(rule, |key| {
                    let oid = backend
                        .get_hash_by_path("master", &("/rules".to_string() + key))
                        .map_err(|_| DataLoaderError::ObjectNotFoundError(key.to_owned()))
                        .unwrap();

                    let res_raw_data = backend.get_obj_by_hash(&oid).unwrap();

                    return RuleWithPath {
                        rule: load_rule(&res_raw_data).unwrap(),
                        path: key.to_owned(),
                    };
                })
                .map_err(|_| DataLoaderError::ObjectNotFoundError((rule).to_owned()))?;

            mem_store
                .link_stor
                .add_link(rule.to_string(), target)
                .unwrap();
        } else {
            return Err(DataLoaderError::ObjectNotFoundError(
                "only support find object by path".to_string(),
            ));
        }

        return Ok(());
    }

    pub fn load_data(&self, cc: &CFGCenterInner) -> Result<Box<MemStorage>, String> {
        let backend = cc.backend.lock().unwrap();
        if backend.is_none() {
            return Err("backend not set".into());
        }
        let backend = backend.as_ref().unwrap();

        let mut mem_store = Box::new(MemStorage {
            rule_stor: RuleStorage::new(),
            res_stor: ResStorage::new(),
            link_stor: LinkStorage::new(),
        });

        let mut nodes_to_visit: Vec<String> = Vec::with_capacity(32);
        nodes_to_visit.push("/links/".to_string());
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in backend.list_dir("master", &parent_node).unwrap() {
                let full_path = parent_node.to_owned() + &cur_node.name;
                if cur_node.name.ends_with("/") {
                    nodes_to_visit.push(full_path);
                    continue;
                }
                let rule_raw_data = backend.get_obj_by_hash(&cur_node.hash).unwrap();
                Self::load_by_link(
                    &cur_node.name,
                    &rule_raw_data,
                    backend.as_ref(),
                    mem_store.as_mut(),
                )
                .unwrap();
            }
        }

        return Ok(mem_store);
    }
}

#[derive(Debug)]

pub struct RuleWithPath {
    pub rule: Rule,
    pub path: String,
}

pub struct RuleStorage {
    storage: BTreeMap<String, Arc<RuleWithPath>>,
}

impl RuleStorage {
    pub fn new() -> Self {
        let storage = BTreeMap::new();
        return Self { storage };
    }

    pub fn load_or_ref_rule<F: Fn(&str) -> RuleWithPath>(
        &mut self,
        path: &str,
        loader: F,
    ) -> Result<(), DataMemStorageError> {
        self.storage
            .entry(path.to_string())
            .or_insert_with(|| Arc::new(loader(path)));
        return Ok(());
    }

    pub fn iter_with_prefix<F: FnMut(&Arc<RuleWithPath>)>(&self, mut cb: F) {
        for (_, v) in self.storage.iter() {
            cb(&v)
        }
    }
}

pub struct KeyValuePair {
    pub content_type: String,
    pub key: String,
    pub value: String,
}

pub struct Resource {
    pub data: Vec<Arc<KeyValuePair>>,
    pub(crate) res_path: Arc<String>,
}

pub struct ResStorage {
    storage: HashMap<String, Resource>,
}

impl ResStorage {
    pub fn new() -> Self {
        return Self {
            storage: HashMap::new(),
        };
    }

    pub fn load_or_ref_res<F>(&mut self, key: &str, loader: F) -> Result<(), DataMemStorageError>
    where
        F: Fn(&str) -> Resource,
    {
        self.storage
            .entry(key.to_owned())
            .or_insert_with(|| loader(key));

        return Ok(());
    }

    pub(crate) fn batch_get_res<F: FnMut(&Resource, &Arc<LinkInfo>, &str) -> bool>(
        &self,
        links: &Vec<&Arc<LinkInfo>>,
        mut cb: F,
    ) {
        for link in links {
            for res_path in &link.reses_path {
                let val = self.storage.get(res_path).unwrap();
                if cb(val, link, res_path.as_ref()) {
                    break;
                }
            }
        }
    }
}

pub struct LinkStorage {
    idx_rule_to_res: BTreeMap<String, Vec<Arc<LinkInfo>>>,
}

impl LinkStorage {
    pub fn new() -> Self {
        return Self {
            idx_rule_to_res: BTreeMap::new(),
        };
    }

    pub fn add_link(
        &mut self,
        link_path: String,
        link: LinkInfo,
    ) -> Result<(), DataMemStorageError> {
        self.idx_rule_to_res
            .entry(link_path)
            .or_default()
            .push(Arc::new(link));
        return Ok(());
    }

    pub(crate) fn batch_get_links<F: FnMut(Vec<&Arc<LinkInfo>>)>(
        &self,
        s: Vec<Arc<RuleWithPath>>,
        mut cb: F,
    ) {
        let mut ret = Vec::new();

        for rule_info in s {
            for target in self
                .idx_rule_to_res
                .get(&rule_info.path)
                .unwrap()
            {
                ret.push(target);
            }
        }
        cb(ret);
    }
}

fn remove_path_type_prefix(i: &str) -> &str {
    // remove `path:`
    return &i[5..];
}
