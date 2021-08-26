mod loader;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::model::object::ObjectID;
use crate::model::{link, res, rule};
use crate::rule_engine::Value;
use crate::storage_backends;
use crate::storage_backends::filesystem;

// mod config_path_spliter;
pub struct CFGCenter<B> {
    backend: B,
    loader: loader::Loader<B>,
    rule_stor: loader::RuleStorage,
    res_stor: loader::ResStorage,
    link_stor: loader::LinkStorage,
}

impl<B> CFGCenter<B>
where
    B: storage_backends::StorageBackend,
{
    pub fn new(backend: B) -> Self {
        return Self {
            backend,
            loader: loader::Loader::new(),
            rule_stor: loader::RuleStorage::new(),
            res_stor: loader::ResStorage::new(),
            link_stor: loader::LinkStorage::new(),
        };
    }

    pub fn get_cfg(&self, ctx: &HashMap<String, Value>, key: &str, prefix: &str) -> Option<Value> {
        let mut act_rules = Vec::new();
        self.rule_stor.iter_with_prefix(prefix, |path, rule| {
            if rule.spec.rule.eval(ctx) {
                act_rules.push(path.to_string());
            }
        });

        let mut ret = None;
        let mut neg_filtered_res_id = Vec::new();
        let mut neg_oids = HashSet::new();

        self.link_stor.batch_get_targets(act_rules, |mut t| {
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

            self.res_stor.batch_get_res(&neg_filtered_res_id, |r| {
                if let Some(v) = r.get(key) {
                    ret = Some(v);
                    return true;
                }
                return false;
            })
        });

        return ret;
    }
}

#[test]
fn test_load_res_and_query() {
    let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_path = project_base_dir
        .join("test")
        .join("mock_data")
        .join("filesystem_backend");
    let backend = filesystem::FilesystemBackend::new(base_path);
    let cc = CFGCenter::new(backend);
    cc.loader.load_data(&cc);


	let mut ctx = HashMap::new();
	ctx.insert("foo".to_string(), Value::Str("123".to_string()));
	ctx.insert("bar".to_string(), Value::Str("456".to_string()));

	for _ in 0..1000000 {

	
		let t = cc.get_cfg(&ctx, "aaa/1/bbb", "/").unwrap();
	}
	
}


