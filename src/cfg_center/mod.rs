mod loader;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::model::object::ObjectID;
use crate::model::{link, res, rule};
use crate::rule_engine::Value;
use crate::storage_backends;
use crate::storage_backends::filesystem;

use self::loader::Resource;

pub struct MemStorage {
    rule_stor: loader::RuleStorage,
    res_stor: loader::ResStorage,
    link_stor: loader::LinkStorage,
}
pub struct CFGCenter {
    backend: Box<dyn storage_backends::StorageBackend>,
    loader: loader::Loader,
    mem_store: Arc<RwLock<Box::<MemStorage>>>,
}

impl CFGCenter {
    pub fn new(backend: Box<dyn storage_backends::StorageBackend>) -> Self {
        let mem_store = Arc::new(RwLock::new(Box::new(MemStorage{
            rule_stor: loader::RuleStorage::new(),
            res_stor: loader::ResStorage::new(),
            link_stor: loader::LinkStorage::new(),
        })));

        return Self {
            backend,
            loader: loader::Loader::new(),
            mem_store,
        };
    }

    pub fn get_cfg(&self, ctx: &HashMap<String, Value>, key: &str) -> Result<(String, String), String> {
        let mut act_rules = Vec::new();

        let mut mem_store = self
        .mem_store
        .read()
        .map_err(|e| e.to_string()).unwrap();

        mem_store.rule_stor.iter_with_prefix( |path, rule| {
            if rule.spec.rule.eval(ctx) {
                act_rules.push(path.to_string());
            }
        });

        let mut ret = Err("No Result".into());
        let mut neg_filtered_res_id = Vec::with_capacity(act_rules.len());
        let mut neg_oids = HashSet::with_capacity(act_rules.len());

        mem_store.link_stor.batch_get_targets(act_rules, |mut t| {
            t.sort_unstable_by(|&a, &b| {
                // safety: infinate value is filtered out when loading links from storage
                if a.pri > b.pri {
                    return Ordering::Less;
                } else if a.pri == b.pri {
                    // if a is neg, no matter what b is, we can put a before b, if b is also neg, the order between a and b does not matter anymore
                    return if a.is_neg {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    };
                } else {
                    return Ordering::Greater;
                }
            });

            // now filter out all the neg items
            for link_tgt in t {
                if link_tgt.is_neg {
                    for target in &link_tgt.target {
                        neg_oids.insert(target.clone());
                    }
                } else {
                    for target in &link_tgt.target {
                        if !neg_oids.contains(target) {
                            neg_filtered_res_id.push(target.clone());
                        }
                    }
                }
            }

            mem_store.res_stor.batch_get_res(&neg_filtered_res_id, |r| {
                if r.key == key {
                    ret = Ok((r.content_type.clone(), r.data.clone()));
                    return true;
                }
                return false;
            })
        });

        return ret;
    }

    pub fn full_load_cfg(&self) {
        self.loader.load_data(&self);
    }


    
}

#[test]
fn test_load_res_and_query() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_path = project_base_dir
        .join("test")
        .join("mock_data")
        .join("filesystem_backend");
    let backend = Box::new(filesystem::FilesystemBackend::new(base_path));
    let cc = CFGCenter::new(backend);
    cc.loader.load_data(&cc);


	let mut ctx = HashMap::new();
	ctx.insert("foo".to_string(), Value::Str("123".to_string()));
	ctx.insert("bar".to_string(), Value::Str("456".to_string()));

	for _ in 0..1000000 {
		let t = cc.get_cfg(&ctx, "my_key").unwrap();
	}
	
}


