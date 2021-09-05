use crate::cfg_center;
use crate::rule_engine::Value;
use crate::storage_backends::{self, filesystem};
use serde_json;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ffi::{CStr, CString};
use std::mem::ManuallyDrop;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
use std::slice;
use std::str::FromStr;

type CFGCenter = cfg_center::CFGCenter;
pub struct Context(HashMap<String, Value>);

pub use crate::cfg_center::ViewMode;

#[no_mangle]
pub extern "C" fn new_config_center_client(
    cfg: *const c_char,
    cb: Option<unsafe extern "C" fn(update_info: *const c_void, usre_data: *const c_void)>,
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

    let mut backend = match build_storage_backend_from_cfg(backend_cfg) {
        Err(_) => return ptr::null(),
        Ok(t) => t,
    };

    let mut cc = cfg_center::CFGCenter::new();

    let cc_for_update_cb = cc.clone();
    if let Some(cb) = cb {
        let t = user_data as usize;
        let rust_cb = Box::new(move |_| {
            cc_for_update_cb.full_load_cfg();
            unsafe { cb(ptr::null(), t as *const c_void) }
        });
        backend.set_update_cb(rust_cb);
    }

    cc.set_backend(backend);
    cc.full_load_cfg();

    let ret = Box::new(cc);

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
    let t = Box::into_raw(Box::new(Context(ret)));
    // println!(">>{:?}", t);
    t
}

#[no_mangle]
pub extern "C" fn free_context(ctx: *mut Context) {
    unsafe { Box::from_raw(ctx) };
}

#[repr(C)]
pub struct ConfigValueReason {
    pub pri: f32,
    pub is_neg: bool,
    pub link_path: *mut c_char,
    pub rule_path: *mut c_char,
    pub res_path: *mut c_char,
}

impl Drop for ConfigValueReason {
    fn drop(&mut self) {
        unsafe {
            CString::from_raw(self.link_path);
            CString::from_raw(self.rule_path);
            CString::from_raw(self.res_path);
        }
    }
}

#[repr(C)]
pub struct ConfigValue {
    key: *mut c_char,
    content_type: *mut c_char,
    value: *mut c_char,
    reason: *mut ConfigValueReason,
}

impl Drop for ConfigValue {
    fn drop(&mut self) {
        unsafe {
            CString::from_raw(self.key);
            CString::from_raw(self.content_type);
            CString::from_raw(self.value);
            if !self.reason.is_null() {
                Box::from_raw(self.reason);
            }
        }
    }
}

#[repr(C)]
pub struct UpdateInfo {
    pub event_cnt: u64,
    pub events: *const UpdateInfoItem,
}

pub struct UpdateInfoItem {

}

#[no_mangle]
pub extern "C" fn get_config(
    cc: *const CFGCenter,
    ctx: *const Context,
    keys: *mut *mut c_char,
    key_cnt: usize,
    view_mode: ViewMode,
    need_explain: u8,
) -> *mut ConfigValue {
    let cc = unsafe { &*cc };

    let ctx = unsafe { &*ctx };

    let key = unsafe {
        let mut ret = Vec::with_capacity(key_cnt);
        for key in slice::from_raw_parts(keys, key_cnt) {
            if let Ok(t) = CStr::from_ptr(*key).to_str() {
                ret.push(t);
            } else {
                return ptr::null_mut();
            }
        }
        ret
    };

    let cc_ref = cc.clone();
    let vs = cc_ref
        .get_cfg(&ctx.0, &key, view_mode, if need_explain==0 {false} else {true})
        .unwrap();

    let mut ret = Vec::with_capacity(key_cnt);
    for v in vs {

        let reason = match v.reason {
            Some(r) => Box::into_raw(Box::new(ConfigValueReason{
                pri: r.link.pri,
                is_neg: r.link.is_neg,
                rule_path: CString::new(&r.link.rule_path[..]).unwrap().into_raw(),
                link_path: CString::new(&r.link.link_path[..]).unwrap().into_raw(),
                res_path: CString::new(&(*r.res_path)[..]).unwrap().into_raw(),
            })),
            None => ptr::null_mut(),
        };

        let item = ConfigValue {
            content_type: CString::new(&v.value.content_type[..]).unwrap().into_raw(),
            key: CString::new(&v.value.key[..]).unwrap().into_raw(),
            value: CString::new(&v.value.value[..]).unwrap().into_raw(),
            reason
        };

        ret.push(item);
    }

    ret.shrink_to_fit();
    let mut ret = ManuallyDrop::new(ret);
    let t = ret.as_mut_ptr();
    return t;
}

#[no_mangle]
pub extern "C" fn free_config_value(v: *mut ConfigValue, n: usize) {
    unsafe { Vec::from_raw_parts(v, n, n) };
}

fn build_storage_backend_from_cfg(
    cfg: &serde_json::Value,
) -> Result<Box<dyn storage_backends::StorageBackend + Send + Sync>, String> {
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
