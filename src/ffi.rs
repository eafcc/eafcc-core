use crate::cfg_center;
use crate::rule_engine::Value;
use crate::storage_backends::{self, filesystem};
use libc;
use serde_json;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

type CFGCenter = RwLock<cfg_center::CFGCenter>;
pub struct Context(HashMap<String, Value>);

#[no_mangle]
pub extern "C" fn new_config_center_client(
    cfg: *const c_char,
    cb: Option<unsafe extern "C" fn(usre_data: *const c_void)>,
    user_data: *const c_void,
) -> *const CFGCenter {

    let t = unsafe {
        assert!(!cfg.is_null());
        CStr::from_ptr(cfg)
    };

    let cfg = match serde_json::from_slice::<serde_json::Value>(t.to_bytes()) {
        Err(_) => return ptr::null(),
        Ok(t) => t,
    };

    let backend_cfg = match cfg.get("storage_backend") {
        None => return ptr::null(),
        Some(t) => t,
    };

    let backend = match build_storage_backend_from_cfg(backend_cfg) {
        Err(_) => return ptr::null(),
        Ok(t) => t,
    };


    if let Some(cb) = cb {
		let t = user_data as usize;
        let rust_cb = Box::new(move |_| unsafe{cb(t as *const c_void )});
        backend.set_update_cb(rust_cb);
    }

    let cc = cfg_center::CFGCenter::new(backend);

    cc.full_load_cfg();

    let ret = Box::new(RwLock::new(cc));

    Box::into_raw(ret)
}

#[no_mangle]
pub extern "C" fn new_context(val: *const c_char) -> *const Context {
    let mut ret = HashMap::new();
    let val = unsafe { CStr::from_ptr(val).to_string_lossy() };
    for part in val.split("\n") {
        if let Some((k, v)) = part.split_once("=") {
            ret.insert(k.trim().to_owned(), Value::Str(v.trim().to_owned()));
        }
    }
    Box::into_raw(Box::new(Context(ret)))
}

#[no_mangle]
pub extern "C" fn free_context(ctx: *mut Context) {
    unsafe { Box::from_raw(ctx) };
}

#[repr(C)]
pub struct ConfigValue {
    content_type: *mut c_char,
    value: *mut c_char,
}

#[no_mangle]
pub extern "C" fn get_config(
    cc: *const CFGCenter,
    ctx: *const Context,
    key: *mut c_char,
) -> *mut ConfigValue {
    let cc = unsafe { &*cc };

    let cc = match cc.read() {
        Err(_) => return ptr::null_mut(),
        Ok(t) => t,
    };

    let ctx = unsafe { &*ctx };

    let key = unsafe { CStr::from_ptr(key).to_string_lossy() };

    let v = cc.get_cfg(&ctx.0, &key).unwrap();

    Box::into_raw(Box::new(ConfigValue {
        content_type: CString::new(v.0).unwrap().into_raw(),
        value: CString::new(v.1).unwrap().into_raw(),
    }))
}

#[no_mangle]
pub extern "C" fn free_config_value(v: *mut ConfigValue) {
    unsafe { Box::from_raw(v) };
}


fn build_storage_backend_from_cfg(
    cfg: &serde_json::Value,
) -> Result<Box<dyn storage_backends::StorageBackend>, String> {
    let cfg = cfg.as_object().ok_or("cfg format error")?;
    let ty = cfg
        .get("type")
        .ok_or("must have a `type` filed for storage_backend")?
        .as_str()
        .ok_or("`storage_backend` must be string")?;

    match ty {
        "filesystem" => {
            let path = cfg
                .get("path")
                .ok_or("filesystem backend must have a `path` filed")?
                .as_str()
                .ok_or("`path` must be string")?;
            let path = PathBuf::from_str(path).or(Err("path invalid"))?;
            let backend = filesystem::FilesystemBackend::new(path);
            Ok(Box::new(backend))
        }
        _ => Err("not supported backend type".to_string()),
    }
}
