mod namespace;
mod cfgindex;
mod mem_store;
mod querier;
mod cfg_center;
mod differ;

use core::time;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;

use crate::rule_engine::Value;
use crate::storage_backends::{filesystem, StorageBackend};

pub use crate::cfg_center::cfg_center::{CFGCenter, ViewMode};
pub use crate::cfg_center::differ::Differ;
pub use crate::cfg_center::namespace::NamespaceScopedCFGCenter;
pub use cfg_center::UpdateNotifyLevel;
pub use querier::CFGResult;

#[test]
fn test_load_res_and_query() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_path = project_base_dir
        .join("test")
        .join("mock_data")
        .join("filesystem_backend");
    let mut backend = Box::new(filesystem::FilesystemBackend::new(base_path));
    let mut cc = CFGCenter::new(backend).unwrap();
    
    let cfg_ns = cc.create_namespace_scoped_cfg_center("/", UpdateNotifyLevel::NotifyWithoutChangedKeysByGlobal, Some(Box::new(|_|{}))).unwrap();

    let cc1 = cfg_ns.clone();
    let cc2 = cfg_ns.clone();

    let t1 = thread::spawn(move || {
        for i in 0..6000000 {
            let mut ctx = HashMap::new();
            ctx.insert("foo".to_string(), Value::Str("123".to_string()));
            ctx.insert("bar".to_string(), Value::Str("456".to_string()));

            let my_key = vec!["my_key", "my_key", "my_key"];
            let t = cc1
                .get_cfg(&ctx, &my_key, ViewMode::OverlaidView, true).unwrap();
            
            if t.len() > 0 {
                println!("{}", t[0].value.value);
            } else {
                println!("no result");
            }
            
            thread::sleep(time::Duration::from_secs(1));
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
            assert!(t.len() == 3)
        }
    });

    t1.join();
    t2.join();

    // thread::sleep(time::Duration::from_secs(10000))
}
