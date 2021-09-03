mod loader;


use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::storage_backends::{StorageBackend, filesystem};
use crate::{rule_engine::Value, storage_backends};


pub struct MemStorage {
    rule_stor: loader::RuleStorage,
    res_stor: loader::ResStorage,
    link_stor: loader::LinkStorage,
}
pub struct CFGCenterInner {
    backend: Mutex<Option<Box<dyn storage_backends::StorageBackend + Send + Sync>>>,
    loader: loader::Loader,
    mem_store: RwLock<Box::<MemStorage>>,
}
#[derive(Clone)]
pub struct CFGCenter(Arc<CFGCenterInner>);

impl CFGCenter {
    pub fn new() -> Self {
        let mem_store = RwLock::new(Box::new(MemStorage{
            rule_stor: loader::RuleStorage::new(),
            res_stor: loader::ResStorage::new(),
            link_stor: loader::LinkStorage::new(),
        }));

        return CFGCenter(Arc::new(CFGCenterInner {
            backend: Mutex::new(None),
            loader: loader::Loader::new(),
            mem_store,
        }));
    }


    pub fn set_backend(&mut self, backend: Box<dyn storage_backends::StorageBackend + Send + Sync>) {
        self.0.backend.lock().unwrap().replace(backend);
    }

    pub fn get_cfg(&self, ctx: &HashMap<String, Value>, keys: &Vec<&str>) -> Result<Vec<(String, String)>, String> {
        let mut act_rules = Vec::new();

        let mem_store = self.0
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

        let mut ret_buf = Vec::with_capacity(keys.len());
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

            for key in keys {
                mem_store.res_stor.batch_get_res(&neg_filtered_res_id, |rs| {
                    for r in rs {
                        if r.key == *key {
                            ret_buf.push((r.content_type.clone(), r.data.clone()));
                            return true;
                        }
                    }
                    ret_buf.push(("".to_string(), "".to_string()));
                    return false;
                })
            }
        });

        ret = Ok(ret_buf);
        return ret;
    }

    pub fn full_load_cfg(&self) {
        let new_conf = self.0.loader.load_data(&self.0).unwrap();
        let mut mem_store = self.0.mem_store.write().unwrap();
        let src = new_conf;
        let dst = &mut *mem_store;
        let _ = std::mem::replace(dst, src);
    }
}



#[test]
fn test_load_res_and_query() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_path = project_base_dir
        .join("test")
        .join("mock_data")
        .join("filesystem_backend");
    let mut backend = Box::new(filesystem::FilesystemBackend::new(base_path));
    let mut cc = CFGCenter::new();

    let cloned_cc_for_update = cc.clone();
    backend.set_update_cb(Box::new(move |_|{
        cloned_cc_for_update.full_load_cfg();
    }));

    cc.set_backend(backend);
    
    cc.full_load_cfg();

    let cc1 = cc.clone();
    let cc2 = cc.clone(); 

    let t1  = thread::spawn(move ||{

        for i in 0..6000000 {
            let mut ctx = HashMap::new();
            ctx.insert("foo".to_string(), Value::Str("123".to_string()));
            ctx.insert("bar".to_string(), Value::Str("456".to_string()));
    
            let my_key = vec!["my_key","my_key","my_key"];
            let t = cc1.get_cfg(&ctx, &my_key).unwrap();
        }
    });
	
    let t2 = thread::spawn(move ||{

        for _ in 0..6000000 {
            let mut ctx = HashMap::new();
            ctx.insert("foo".to_string(), Value::Str("123".to_string()));
            ctx.insert("bar".to_string(), Value::Str("456".to_string()));
            
            let my_key = vec!["my_key","my_key","my_key"];
            let t = cc2.get_cfg(&ctx, &my_key).unwrap();
        }
    });

    t1.join();
    t2.join();
    
    // thread::sleep(time::Duration::from_secs(10000))
}


