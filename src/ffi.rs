use crate::cfg_center::{self, CFGResult, Differ, NamespaceScopedCFGCenter, UpdateNotifyLevel};
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
use std::sync::Arc;

type CFGCenter = cfg_center::CFGCenter;
pub struct WhoAmI(HashMap<String, Value>);

pub use crate::cfg_center::ViewMode;

#[no_mangle]
pub extern "C" fn new_config_center_client(cfg: *const c_char) -> *const CFGCenter {
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

    if let Ok(cc) = cfg_center::CFGCenter::new(backend) {
        let ret = Box::new(cc);
        return Box::into_raw(ret);
    } else {
        // TODO set error
        return ptr::null_mut();
    }
}

#[no_mangle]
pub extern "C" fn free_config_center(cc: *mut CFGCenter) {
    unsafe { Box::from_raw(cc) };
}

#[no_mangle]
pub extern "C" fn create_namespace(
    cc: *const CFGCenter,
    namespace: *const c_char,
    notify_level: UpdateNotifyLevel,
    cb: Option<unsafe extern "C" fn(differ: *const Differ, usre_data: *const c_void)>,
    user_data: *const c_void,
) -> *const NamespaceScopedCFGCenter {
    let cc = unsafe {
        assert!(!cc.is_null());
        &*cc
    };

    let namespace = unsafe {
        assert!(!namespace.is_null());
        if let Ok(t) = CStr::from_ptr(namespace).to_str() {
            t
        } else {
            return ptr::null_mut();
        }
    };

    // `*const void` is not `Sned`, convert it to a normal number
    let uintptr = user_data as usize;
    let callback = if let Some(cb) = cb {
        Some(Box::new(move |differ: &Differ| unsafe {
            cb(differ as *const Differ, uintptr as *const c_void);
        }) as Box<dyn Fn(&Differ) + Send + Sync>)
    } else {
        None
    };

    if let Ok(ns) = cc.create_namespace_scoped_cfg_center(namespace, notify_level, callback) {
        return Arc::into_raw(ns);
    } else {
        return ptr::null_mut();
    }
}

#[no_mangle]
pub extern "C" fn free_namespace(ns: *const NamespaceScopedCFGCenter) {
    unsafe { Arc::from_raw(ns) };
}

#[no_mangle]
pub extern "C" fn new_context(val: *const c_char) -> *const WhoAmI {
    let mut ret = HashMap::new();
    let val = unsafe { CStr::from_ptr(val).to_string_lossy() };
    for part in val.split("\n") {
        if let Some((k, v)) = part.split_once("=") {
            ret.insert(k.trim().to_owned(), Value::Str(v.trim().to_owned()));
        }
    }
    let t = Box::into_raw(Box::new(WhoAmI(ret)));
    t
}

#[no_mangle]
pub extern "C" fn free_context(ctx: *mut WhoAmI) {
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

// #[repr(C)]
// pub struct UpdateInfo {
//     pub event_cnt: u64,
//     pub events: *const UpdateInfoItem,
// }

// pub struct UpdateInfoItem {
//     pub event_type: UpdateInfoEventType,
//     pub key: *mut char,
// }

#[no_mangle]
pub extern "C" fn get_config(
    ns: *const NamespaceScopedCFGCenter,
    whoami: *const WhoAmI,
    keys: *mut *mut c_char,
    key_cnt: usize,
    view_mode: ViewMode,
    need_explain: u8,
) -> *mut ConfigValue {
    let ns = unsafe {
        assert!(!ns.is_null());
        &*ns
    };


    let (whoami, keys) = match convert_get_cfg_input_value(whoami, keys, key_cnt) {
        Ok((whoami, keys)) => (whoami, keys),
        Err(_ ) => {return ptr::null_mut()},
    };

    let values = match ns
            .get_cfg(
                &whoami.0,
                &keys,
                view_mode,
                if need_explain == 0 { false } else { true },
            ){
                Ok(values) => values,
                Err(_) => {return ptr::null_mut()},
            };

    match convert_get_cfg_output_value(values) {
        Ok(p) => return p,
        Err(_) => return ptr::null_mut(),
    }
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

#[inline(always)]
fn convert_get_cfg_input_value<'a>(
    whoami: *const WhoAmI,
    keys: *mut *mut c_char,
    key_cnt: usize,
) -> Result<(&'a WhoAmI, Vec<&'a str>), String>{
    let whoami = unsafe {
        assert!(!whoami.is_null());
        &*whoami
    };

    let key = unsafe {
        let mut ret = Vec::with_capacity(key_cnt);
        for key in slice::from_raw_parts(keys, key_cnt) {
            match CStr::from_ptr(*key).to_str() {
                Ok(t) => {ret.push(t)},
                Err(e) => {return Err(e.to_string())}
            }
        }
        ret
    };

    return Ok((whoami, key))
}


#[inline(always)]
fn convert_get_cfg_output_value(
    values: Vec<CFGResult>,
) -> Result<*mut ConfigValue, String>{
    let mut ret = Vec::with_capacity(values.len());
    for v in values {
        let reason = match v.reason {
            Some(r) => Box::into_raw(Box::new(ConfigValueReason {
                pri: r.pri,
                is_neg: r.is_neg,
                rule_path: CString::new(&r.rule_path[..]).unwrap().into_raw(),
                link_path: CString::new(&r.link_path[..]).unwrap().into_raw(),
                res_path: CString::new(&r.abs_res_path[..]).unwrap().into_raw(),
            })),
            None => ptr::null_mut(),
        };

        let item = ConfigValue {
            content_type: CString::new(&v.value.content_type[..]).unwrap().into_raw(),
            key: CString::new(&v.value.key[..]).unwrap().into_raw(),
            value: CString::new(&v.value.value[..]).unwrap().into_raw(),
            reason,
        };

        ret.push(item);
    }

    ret.shrink_to_fit();
    let mut ret = ManuallyDrop::new(ret);
    let t = ret.as_mut_ptr();
    return Ok(t);
}


// #[no_mangle]
// pub extern "C" fn differ_get_from_old(v: *mut ConfigValue, n: usize) {
//     unsafe { Vec::from_raw_parts(v, n, n) };
// }


#[no_mangle]
pub extern "C" fn differ_get_from_old(
    differ: *const Differ,
    whoami: *const WhoAmI,
    keys: *mut *mut c_char,
    key_cnt: usize,
    view_mode: ViewMode,
    need_explain: u8,
) -> *mut ConfigValue {
    let differ = unsafe {
        assert!(!differ.is_null());
        &*differ
    };

    let (whoami, keys) = match convert_get_cfg_input_value(whoami, keys, key_cnt) {
        Ok((whoami, keys)) => (whoami, keys),
        Err(_ ) => {return ptr::null_mut()},
    };

    let values = match differ
            .get_from_old(
                &whoami.0,
                &keys,
                view_mode,
                if need_explain == 0 { false } else { true },
            ){
                Ok(values) => values,
                Err(_) => {return ptr::null_mut()},
            };

    match convert_get_cfg_output_value(values) {
        Ok(p) => return p,
        Err(_) => return ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn differ_get_from_new(
    differ: *const Differ,
    whoami: *const WhoAmI,
    keys: *mut *mut c_char,
    key_cnt: usize,
    view_mode: ViewMode,
    need_explain: u8,
) -> *mut ConfigValue {
    let differ = unsafe {
        assert!(!differ.is_null());
        &*differ
    };

    let (whoami, keys) = match convert_get_cfg_input_value(whoami, keys, key_cnt) {
        Ok((whoami, keys)) => (whoami, keys),
        Err(_ ) => {return ptr::null_mut()},
    };

    let values = match differ
            .get_from_new(
                &whoami.0,
                &keys,
                view_mode,
                if need_explain == 0 { false } else { true },
            ){
                Ok(values) => values,
                Err(_) => {return ptr::null_mut()},
            };

    match convert_get_cfg_output_value(values) {
        Ok(p) => return p,
        Err(_) => return ptr::null_mut(),
    }
}