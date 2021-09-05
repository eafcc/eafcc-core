mod loader;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;

use crate::model::link::LinkInfo;
use crate::model::object::ObjectID;
use crate::storage_backends::{filesystem, StorageBackend};
use crate::{rule_engine::Value, storage_backends};

use self::loader::{KeyValuePair, RuleWithPath};

pub struct MemStorage {
    rule_stor: loader::RuleStorage,
    res_stor: loader::ResStorage,
    link_stor: loader::LinkStorage,
}
pub struct CFGCenterInner {
    backend: Mutex<Option<Box<dyn storage_backends::StorageBackend + Send + Sync>>>,
    loader: loader::Loader,
    mem_store: RwLock<Box<MemStorage>>,
}

pub struct LinkAndResInfo {
    pub link: Arc<LinkInfo>,
    pub res_path: Arc<String>,
}

pub struct CFGResult {
    pub reason: Option<LinkAndResInfo>,
    pub value: Arc<KeyValuePair>,
}

#[repr(u32)]
pub enum ViewMode {
    OverlaidView,
    AllLinkedResView,
}

#[derive(Clone)]
pub struct CFGCenter(Arc<CFGCenterInner>);

impl CFGCenter {
    pub fn new() -> Self {
        let mem_store = RwLock::new(Box::new(MemStorage {
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

    pub fn set_backend(
        &mut self,
        backend: Box<dyn storage_backends::StorageBackend + Send + Sync>,
    ) {
        self.0.backend.lock().unwrap().replace(backend);
    }

    pub fn get_cfg(
        &self,
        ctx: &HashMap<String, Value>,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>, String> {
        let mut act_rules = Vec::new();
        let mem_store = self.0.mem_store.read().map_err(|e| e.to_string()).unwrap();

        mem_store.rule_stor.iter_with_prefix(|rule| {
            if rule.rule.spec.rule.eval(ctx) {
                act_rules.push(rule.clone());
            }
        });

        let mut ret = Err("No Result".into());



        let batch_link_handler = match view_mode {
            ViewMode::OverlaidView => fetch_res_by_overlaid_view,
            ViewMode::AllLinkedResView => fetch_res_by_all_linked_res_view,
        };

        mem_store
            .link_stor
            .batch_get_links(act_rules, |links|{
                let t = batch_link_handler(&*mem_store, keys, links, need_explain);
                ret = Ok(t);
            });
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

fn fetch_res_by_overlaid_view(
    mem_store: &MemStorage,
    keys: &Vec<&str>,
    mut links: Vec<&Arc<LinkInfo>>,
    need_explain: bool,
) -> Vec<CFGResult> {
    let mut ret_buf = Vec::with_capacity(keys.len());

    links.sort_unstable_by(|a, b| {
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

    for key in keys {
        mem_store
            .res_stor
            .batch_get_res(&links, |reses_of_a_link, link, res_path| {
                for res in &reses_of_a_link.data {
                    if res.key == *key {
                        if !link.is_neg {
                            let reason = if need_explain {
                                Some(LinkAndResInfo {
                                    link: link.clone(),
                                    res_path: reses_of_a_link.res_path.clone(),
                                })
                            } else {
                                None
                            };

                            ret_buf.push(CFGResult {
                                reason,
                                value: res.clone(),
                            });
                        }
                        return true;
                    }
                }
                return false;
            })
    }
    return ret_buf;
}


fn fetch_res_by_all_linked_res_view(
    mem_store: &MemStorage,
    keys: &Vec<&str>,
    mut links: Vec<&Arc<LinkInfo>>,
    need_explain: bool,
) -> Vec<CFGResult> {
    let mut ret_buf = Vec::with_capacity(keys.len());

    for key in keys {
        mem_store
            .res_stor
            .batch_get_res(&links, |reses_of_a_link, link, res_path| {
                for res in &reses_of_a_link.data {
                    if res.key == *key {
                        let reason = if need_explain {
                            Some(LinkAndResInfo {
                                link: link.clone(),
                                res_path: reses_of_a_link.res_path.clone(),
                            })
                        } else {
                            None
                        };
                        unsafe {
                            // safety: we can ensure only one of the closure will be called, so ret_buf can be mut borrowed in to closures
                            let mut t = &mut *(&ret_buf as *const Vec<CFGResult>
                                as *mut Vec<CFGResult>);
                            t.push(CFGResult {
                                reason,
                                value: res.clone(),
                            });
                        }
                    }
                }
                return false;
            })
    }

    return ret_buf;
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
    backend.set_update_cb(Box::new(move |_| {
        cloned_cc_for_update.full_load_cfg();
    }));

    cc.set_backend(backend);

    cc.full_load_cfg();

    let cc1 = cc.clone();
    let cc2 = cc.clone();

    let t1 = thread::spawn(move || {
        for i in 0..6000000 {
            let mut ctx = HashMap::new();
            ctx.insert("foo".to_string(), Value::Str("123".to_string()));
            ctx.insert("bar".to_string(), Value::Str("456".to_string()));

            let my_key = vec!["my_key", "my_key", "my_key"];
            let t = cc1
                .get_cfg(&ctx, &my_key, ViewMode::OverlaidView, true)
                .unwrap();
        }
    });

    let t2 = thread::spawn(move || {
        for _ in 0..6000000 {
            let mut ctx = HashMap::new();
            ctx.insert("foo".to_string(), Value::Str("123".to_string()));
            ctx.insert("bar".to_string(), Value::Str("456".to_string()));

            let my_key = vec!["my_key", "my_key", "my_key"];
            let t = cc2
                .get_cfg(&ctx, &my_key, ViewMode::OverlaidView, true)
                .unwrap();
        }
    });

    t1.join();
    t2.join();

    // thread::sleep(time::Duration::from_secs(10000))
}
