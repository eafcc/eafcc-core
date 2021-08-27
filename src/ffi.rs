use crate::cfg_center::CFGCenter;
use crate::rule_engine::Value;
use std::collections::HashMap;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use std::ptr;
use std::str::FromStr;
use serde_json;
use std::sync::{RwLock, Arc};
use crate::storage_backends::{self, filesystem};
use std::path::PathBuf;
use libc;

#[no_mangle]
fn new_config_center_client(cfg: *const c_char) -> *const RwLock<CFGCenter>{

	let t = unsafe{
		assert!(!cfg.is_null());
		CStr::from_ptr(cfg)
	};

	let cfg = match serde_json::from_slice::<serde_json::Value>(t.to_bytes()){
		Err(_) => return ptr::null(),
		Ok(t) => t,
	};

	let backend_cfg = match cfg.get("storage_backend"){
		None => return ptr::null(),
		Some(t) => t,
	};

	let backend = match build_storage_backend_from_cfg(backend_cfg){
		Err(_) => return ptr::null(),
		Ok(t) => t,
	};

    let cc = CFGCenter::new(backend);
	let ret = Box::new(RwLock::new(cc));
	
	Box::into_raw(ret)
	
}

#[no_mangle]
fn get_config(cc: *mut RwLock<CFGCenter>, ctx: *mut HashMap<String, Value>) {

	let cc = unsafe{
		Box::from_raw(cc)
	};

	let cc = match cc.read() {
		Err(_) => return,
		Ok(t) => t,
	};

	let ctx = unsafe{
		Box::from_raw(ctx)
	};

	
	cc.get_cfg(&ctx, "aaa/1/bbb", "/").unwrap();

}


fn build_storage_backend_from_cfg(cfg: &serde_json::Value) -> Result<Box<dyn storage_backends::StorageBackend>, String> {
	let cfg = cfg.as_object().ok_or("cfg format error")?;
	let ty = cfg.get("type").ok_or("must have a `type` filed for storage_backend")?.as_str().ok_or("`storage_backend` must be string")?;

	match ty {
		"filesystem" => {
			let path = cfg.get("path").ok_or("filesystem backend must have a `path` filed")?.as_str().ok_or("`path` must be string")?;
			let path = PathBuf::from_str(path).or(Err("path invalid"))?;
			let backend = filesystem::FilesystemBackend::new(path);
			Ok(Box::new(backend))
		}
		_ => {
			Err("not supported backend type".to_string())
		}
	}
}