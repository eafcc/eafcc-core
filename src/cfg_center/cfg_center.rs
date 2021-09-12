
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;

use crate::storage_backends::{StorageBackend, StorageChangeEvent, filesystem};
use crate::{rule_engine::Value, storage_backends};


use super::mem_store::MemStorage;
use super::namespace::NamespaceScopedCFGCenter;
use super::querier::CFGResult;


pub struct CFGCenterInner {
    backend: Arc<dyn storage_backends::StorageBackend + Send + Sync>,
    namespaces: Mutex<HashMap<String, Arc<NamespaceScopedCFGCenter>>>,
}

impl CFGCenterInner {
	pub fn get_namespace_scoped_cfg_center(&self, namespace: &str) -> Result<Arc<NamespaceScopedCFGCenter>, String> {
		let namespaces = self.namespaces.lock().or(Err("get lock error".to_string()))?;
		if let Some(t) = namespaces.get(namespace) {
			return Ok(t.clone())
		}
		return Err("no namespace found".to_string())
	}
	
	pub fn create_namespace_scoped_cfg_center(&self, namespace: &str) -> Result<Arc<NamespaceScopedCFGCenter>, String> {

		if !namespace.starts_with("/") || !namespace.ends_with("/") {
			return Err("namespace must starts and end with `/`".to_owned())
		}
		
		let version = self.backend.get_current_version().map_err(|e|e.to_string())?;

		let mem_store = Box::new(MemStorage::new(self.backend.as_ref(), namespace, &version));

		let v = Arc::new(NamespaceScopedCFGCenter::new(namespace, mem_store));

		let mut namespaces = self.namespaces.lock().or(Err("get lock error".to_string()))?;
		namespaces.insert(namespace.to_owned(), v.clone());
		Ok(v)
	}

	fn update_callback(&self, e: StorageChangeEvent) {

	}
}



#[repr(u32)]
pub enum ViewMode {
    OverlaidView,
    AllLinkedResView,
}

#[derive(Clone)]
pub struct CFGCenter(Arc<CFGCenterInner>);

impl CFGCenter {
    pub fn new(backend: Box<dyn storage_backends::StorageBackend + Send + Sync>) -> Self {

		let inner = Arc::new(CFGCenterInner {
            backend: Arc::from(backend),
            namespaces: Mutex::new(HashMap::new()),
        });

		let inner_for_capture = inner.clone();
		inner.backend.set_update_cb(Box::new(move |x| {inner_for_capture.update_callback(x)}));

        let t =  CFGCenter(inner);
		t
    }


	pub fn get_namespace_scoped_cfg_center(&self, namespace: &str) -> Result<Arc<NamespaceScopedCFGCenter>, String> {
		return self.0.get_namespace_scoped_cfg_center(namespace);
	}
	
	pub fn create_namespace_scoped_cfg_center(&self, namespace: &str) -> Result<Arc<NamespaceScopedCFGCenter>, String> {
		return self.0.create_namespace_scoped_cfg_center(namespace);
	}


  


}

