// mod loader;

// use std::cmp::Ordering;
// use std::collections::{HashMap, HashSet};
// use std::marker::PhantomData;
// use std::path::PathBuf;

// use crate::model::object::ObjectID;
// use crate::model::{link, res, rule};
// use crate::rule_engine::Value;
// use crate::storage_backends;
// use crate::storage_backends::filesystem;

// #[bench]
// fn test_load_res_and_query() {
//     let project_base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//     let base_path = project_base_dir
//         .join("test")
//         .join("mock_data")
//         .join("filesystem_backend");
//     let backend = filesystem::FilesystemBackend::new(base_path);
//     let cc = CFGCenter::new(backend);
//     cc.loader.load_data(&cc);


// 	let mut ctx = HashMap::new();
// 	ctx.insert("foo".to_string(), Value::Str("123".to_string()));
// 	ctx.insert("bar".to_string(), Value::Str("456".to_string()));

// 	for _ in 0..1000000 {

	
// 		let t = cc.get_cfg(&ctx, "aaa/1/bbb", "/").unwrap();
// 	}
	
// }


