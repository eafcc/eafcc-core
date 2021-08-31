use crate::model::link::{LinkMeta, LinkSpec, LinkTarget};
use crate::model::res::{ResMeta, ResSpec};
use crate::model::rule::{self, load_rule};
use crate::storage_backends;

use crate::error::{DataLoaderError, DataMemStorageError};
use crate::model::object::{ObjectID, ObjectIDRef};
use crate::rule_engine::{Rule, RuleMeta, RuleSpec, Value};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::cfg_center::CFGCenter;
use crate::model::RootCommon;
use std::cell::RefCell;
use std::collections::btree_map::Range;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::RwLock;

pub struct Loader {}

impl Loader {
    pub fn new() -> Self {
        return Self {};
    }

    pub fn load_by_link(
        link_id: &str,
        link_data: &[u8],
        cc: &CFGCenter,
    ) -> Result<(), DataLoaderError> {
        let root = serde_json::from_slice::<RootCommon>(link_data)?;
        let meta = serde_json::from_value::<LinkMeta>(root.meta)?;
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

        let mut mem_store = cc.mem_store.write().map_err(|e| e.to_string()).unwrap();

        // load all res that this rule depends on
        for res in spec.reses.iter() {
            if res.starts_with("path:/") && res.len() > 6 {
                let res = &res[5..];
                let oid = cc
                    .backend
                    .get_hash_by_path("master", &("/reses".to_string() + &res))
                    .map_err(|_| DataLoaderError::ObjectNotFoundError(res.to_owned()))?;

                // the following function call will increase the reference counter

                mem_store
                    .res_stor
                    .load_or_ref_res(&oid, |key| {
                        let res_raw_data = cc.backend.get_obj_by_hash(key).unwrap();
                        let root = serde_json::from_slice::<RootCommon>(&res_raw_data).unwrap();
                        let meta = serde_json::from_value::<ResMeta>(root.meta).unwrap();
                        let spec = serde_json::from_value::<ResSpec>(root.spec).unwrap();
                        return Resource {
                            content_type: spec.content_type,
                            key: spec.key,
                            data: spec.data,
                        };
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
                    let oid = cc
                        .backend
                        .get_hash_by_path("master", &("/rules".to_string() + key))
                        .map_err(|_| DataLoaderError::ObjectNotFoundError(key.to_owned()))
                        .unwrap();

                    let res_raw_data = cc.backend.get_obj_by_hash(&oid).unwrap();

                    return load_rule(&res_raw_data).unwrap();
                })
                .map_err(|_| DataLoaderError::ObjectNotFoundError((rule).to_owned()))?;

            mem_store.link_stor.add_rule(rule.to_string(), target);
        } else {
            return Err(DataLoaderError::ObjectNotFoundError(
                "only support find object by path".to_string(),
            ));
        }

        return Ok(());
    }

    pub fn load_data(&self, cc: &CFGCenter) {
        let mut nodes_to_visit: Vec<String> = Vec::with_capacity(32);
        nodes_to_visit.push("/links/".to_string());
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in cc.backend.list_dir("master", &parent_node).unwrap() {
                let full_path = parent_node.to_owned() + &cur_node.name;
                if cur_node.name.ends_with("/") {
                    nodes_to_visit.push(full_path);
                    continue;
                }
                let rule_raw_data = cc.backend.get_obj_by_hash(&cur_node.hash).unwrap();
                Self::load_by_link(&cur_node.name, &rule_raw_data, cc).unwrap();
            }
        }
    }
}

#[derive(Debug)]
struct StorageEntry<T> {
    counter: usize,
    data: T,
}

pub struct RuleStorage {
    storage: RefCell<BTreeMap<String, StorageEntry<Rule>>>,
}

impl RuleStorage {
    pub fn new() -> Self {
        let storage = RefCell::new(BTreeMap::new());
        return Self { storage };
    }

    pub fn load_or_ref_rule<F: Fn(&str) -> Rule>(
        &self,
        path: &str,
        loader: F,
    ) -> Result<(), DataMemStorageError> {
        let mut storage = self.storage.borrow_mut();
        let e = storage.entry(path.to_string())
            .or_insert_with(|| StorageEntry {
                counter: 0,
                data: loader(path),
            });
        e.counter += 1;

        return Ok(());
    }

    pub fn release_rule(&self, path: &str) -> Result<(), DataMemStorageError> {
        if let Some(e) = self.storage.borrow_mut().get_mut(path) {
            if e.counter == 1 {
                self.storage.borrow_mut().remove(path);
            }
        }
        Ok(())
    }

    pub fn iter_with_prefix<F: FnMut(&String, &Rule)>(&self, mut cb: F) {
        for (k, v) in self.storage.borrow().iter() {
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
    storage: RwLock<HashMap<Vec<u8>, StorageEntry<Resource>>>,
}

impl ResStorage {
    pub fn new() -> Self {
        return Self {
            storage: RwLock::new(HashMap::new()),
        };
    }

    pub fn load_or_ref_res<F>(&self, key: ObjectIDRef, loader: F) -> Result<(), DataMemStorageError>
    where
        F: Fn(ObjectIDRef) -> Resource,
    {
        let mut storage = self
            .storage
            .write()
            .map_err(|e| DataMemStorageError::CustomError(e.to_string()))?;
        let e = storage.entry(key.to_vec()).or_insert_with(|| StorageEntry {
            counter: 0,
            data: loader(key),
        });
        e.counter += 1;

        return Ok(());
    }

    pub fn release_res(&self, key: ObjectIDRef) -> Result<(), DataMemStorageError> {
        let mut storage = self
            .storage
            .write()
            .map_err(|e| DataMemStorageError::CustomError(e.to_string()))?;
        if let Some(e) = storage.get_mut(key) {
            if e.counter == 1 {
                storage.remove(key);
            }
        }
        Ok(())
    }

    pub fn batch_get_res<F: FnMut(&Resource) -> bool>(&self, s: &Vec<ObjectID>, mut cb: F) {
        // let mut ret = Vec::new();

        let storage = self
            .storage
            .read()
            .map_err(|e| DataMemStorageError::CustomError(e.to_string()))
            .unwrap();

        for oid in s {
            let val = storage.get(oid).unwrap();
            if cb(&val.data) {
                break;
            }
        }
    }
}

pub struct LinkStorage {
    idx_rule_to_res: RwLock<BTreeMap<String, Vec<LinkTarget>>>,
}

impl LinkStorage {
    pub fn new() -> Self {
        return Self {
            idx_rule_to_res: RwLock::new(BTreeMap::new()),
        };
    }

    pub fn add_rule(&self, link_path: String, link: LinkTarget) -> Result<(), DataMemStorageError> {
        let mut storage = self
            .idx_rule_to_res
            .write()
            .map_err(|e| DataMemStorageError::CustomError(e.to_string()))?;

        storage.entry(link_path).or_default().push(link);
        return Ok(());
    }

    pub fn batch_get_targets<F: FnMut(Vec<&LinkTarget>)>(&self, s: Vec<String>, mut cb: F) {
        let mut ret = Vec::new();

        let storage = self
            .idx_rule_to_res
            .read()
            .map_err(|e| DataMemStorageError::CustomError(e.to_string()))
            .unwrap();

        for rule_name in s {
            for target in storage.get(&rule_name).unwrap() {
                ret.push(target);
            }
        }
        cb(ret);
    }
}
