use crate::model::link::{LinkMeta, LinkSpec, LinkTarget};
use crate::model::res::{ResMeta, ResSpec};
use crate::model::rule::load_rule;
use crate::storage_backends;

use crate::error::{DataLoaderError, DataMemStorageError};
use crate::model::object::{ObjectID, ObjectIDRef};
use crate::rule_engine::Rule;
use serde_json;

use crate::model::RootCommon;

use std::collections::{BTreeMap, HashMap};

use super::{CFGCenterInner, MemStorage};

pub struct Loader {}

impl Loader {
    pub fn new() -> Self {
        return Self {};
    }

    pub fn load_by_link(
        link_id: &str,
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

        let mut target = LinkTarget {
            pri: spec.pri,
            is_neg: spec.is_neg,
            target: Vec::new(),
            link_id: link_id.to_string(),
        };

        // load all res that this rule depends on
        for res in spec.reses.iter() {
            if res.starts_with("path:/") && res.len() > 6 {
                let res = &res[5..];
                let oid = backend
                    .get_hash_by_path("master", &("/reses".to_string() + &res))
                    .map_err(|_| DataLoaderError::ObjectNotFoundError(res.to_owned()))?;

                // the following function call will increase the reference counter

                mem_store
                    .res_stor
                    .load_or_ref_res(&oid, |key| {
                        let res_raw_data = backend.get_obj_by_hash(key).unwrap();
                        let root = serde_json::from_slice::<RootCommon>(&res_raw_data).unwrap();
                        let _meta = serde_json::from_value::<ResMeta>(root.meta).unwrap();
                        let spec = serde_json::from_value::<ResSpec>(root.spec).unwrap();

                        let ret: Vec<_> = spec
                            .0
                            .into_iter()
                            .map(|e| Resource {
                                content_type: e.content_type,
                                key: e.key,
                                data: e.data,
                            })
                            .collect();
                        return ret;
                    })
                    .map_err(|_| DataLoaderError::ObjectNotFoundError(res.to_owned()))?;

                target.target.push(oid);
            } else {
                return Err(DataLoaderError::ObjectNotFoundError(
                    "only support find object by path".to_string(),
                ));
            }
        }

        // the following function call will increase the reference counter
        if spec.rule.starts_with("path:/") && spec.rule.len() > 6 {
            let rule = &spec.rule[5..];
            mem_store
                .rule_stor
                .load_or_ref_rule(rule, |key| {
                    let oid = backend
                        .get_hash_by_path("master", &("/rules".to_string() + key))
                        .map_err(|_| DataLoaderError::ObjectNotFoundError(key.to_owned()))
                        .unwrap();

                    let res_raw_data = backend.get_obj_by_hash(&oid).unwrap();

                    return load_rule(&res_raw_data).unwrap();
                })
                .map_err(|_| DataLoaderError::ObjectNotFoundError((rule).to_owned()))?;

            mem_store
                .link_stor
                .add_rule(rule.to_string(), target)
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
struct StorageEntry<T> {
    data: T,
}

pub struct RuleStorage {
    storage: BTreeMap<String, StorageEntry<Rule>>,
}

impl RuleStorage {
    pub fn new() -> Self {
        let storage = BTreeMap::new();
        return Self { storage };
    }

    pub fn load_or_ref_rule<F: Fn(&str) -> Rule>(
        &mut self,
        path: &str,
        loader: F,
    ) -> Result<(), DataMemStorageError> {
        self.storage
            .entry(path.to_string())
            .or_insert_with(|| StorageEntry { data: loader(path) });
        return Ok(());
    }

    pub fn iter_with_prefix<F: FnMut(&String, &Rule)>(&self, mut cb: F) {
        // println!("=======");
        for (k, v) in self.storage.iter() {
            cb(k, &v.data)
        }
    }
}

pub struct Resource {
    pub content_type: String,
    pub key: String,
    pub data: String,
}

pub struct ResStorage {
    storage: HashMap<Vec<u8>, StorageEntry<Vec<Resource>>>,
}

impl ResStorage {
    pub fn new() -> Self {
        return Self {
            storage: HashMap::new(),
        };
    }

    pub fn load_or_ref_res<F>(
        &mut self,
        key: ObjectIDRef,
        loader: F,
    ) -> Result<(), DataMemStorageError>
    where
        F: Fn(ObjectIDRef) -> Vec<Resource>,
    {
        self.storage
            .entry(key.to_vec())
            .or_insert_with(|| StorageEntry { data: loader(key) });

        return Ok(());
    }

    pub fn batch_get_res<F: FnMut(&Vec<Resource>) -> bool>(&self, s: &Vec<ObjectID>, mut cb: F) {
        for oid in s {
            let val = self.storage.get(oid).unwrap();
            if cb(&val.data) {
                break;
            }
        }
    }
}

pub struct LinkStorage {
    idx_rule_to_res: BTreeMap<String, Vec<LinkTarget>>,
}

impl LinkStorage {
    pub fn new() -> Self {
        return Self {
            idx_rule_to_res: BTreeMap::new(),
        };
    }

    pub fn add_rule(
        &mut self,
        link_path: String,
        link: LinkTarget,
    ) -> Result<(), DataMemStorageError> {
        self.idx_rule_to_res
            .entry(link_path)
            .or_default()
            .push(link);
        return Ok(());
    }

    pub fn batch_get_targets<F: FnMut(Vec<&LinkTarget>)>(&self, s: Vec<String>, mut cb: F) {
        let mut ret = Vec::new();

        for rule_name in s {
            for target in self.idx_rule_to_res.get(&rule_name).unwrap() {
                ret.push(target);
            }
        }
        cb(ret);
    }
}
