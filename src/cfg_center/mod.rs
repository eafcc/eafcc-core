mod loader;

use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::storage_backends;
use crate::model::{rule,res, link};
use crate::storage_backends::filesystem;
use crate::rule_engine::Value;

pub struct CFGCenter<B> {
	backend: B,
	loader: loader::Loader<B>, 
	rule_stor: loader::RuleStorage,
	res_stor: loader::ResStorage,
	link_stor: loader::LinkStorage,
}

impl<B> CFGCenter<B> where B:storage_backends::StorageBackend{

	pub fn new(backend: B) -> Self{
		return Self{
			backend,
			loader: loader::Loader::new(), 
			rule_stor: loader::RuleStorage::new(),
			res_stor: loader::ResStorage::new(),
			link_stor: loader::LinkStorage::new(),
		}
	}

	pub fn get_cfg(&self, ctx: HashMap<String, Value>, key:&str, prefix: &str) {
		let mut act_rules = Vec::new();
		self.rule_stor.iter_with_prefix(prefix, |path,rule| {
			if rule.spec.rule.eval(&ctx) {
				act_rules.push(path.to_string());
			}
		});

		self.link_stor.batch_get_targets(act_rules, |t| {
			
		})


	}

}


#[test]
fn test_load_res() {
	let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let base_path = project_base_dir.join("test").join("mock_data").join("filesystem_backend");
	let backend = filesystem::FilesystemBackend::new(base_path);
	let cc = CFGCenter::new(backend);
	cc.loader.load_data(&cc);
}

