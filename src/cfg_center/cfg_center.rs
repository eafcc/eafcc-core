use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread;

use crate::storage_backends::{filesystem, StorageBackend, StorageChangeEvent};
use crate::{rule_engine::Value, storage_backends};

use super::differ::Differ;
use super::mem_store::MemStorage;
use super::namespace::{NamespaceScopedCFGCenter, UpdateEventItem};
use super::querier::CFGResult;

use crate::error::{Result, CCLibError};

pub struct CFGCenterInner {
	// from the config reader's view, you can not change the internal state of the backend storage system
	// so it's ok and should make the backend not mutable
    backend: Arc<dyn storage_backends::StorageBackend + Send + Sync>,
    namespaces: Mutex<HashMap<String, Arc<NamespaceScopedCFGCenter>>>,
	current_version: Mutex<String>,
}

#[derive(PartialEq, Clone, Copy)]
#[repr(u32)]
pub enum UpdateNotifyLevel {
	// No Notify At All
    NoNotify,
	// tell you changes occured, can be triggered by changes in other namespace, and won't tell you which key has changed.
	// this is the fastest level, require little extra cpu power.
	NotifyWithoutChangedKeysByGlobal,
	// tell you changes occured in this namespace, but won't tell you which key has changed
	// this is the middle level, require some extra cpu power, depending on the config set scale.
    NotifyWithoutChangedKeysInNamespace,
	// tell you which keys MAYBE changed, so you can clear your cache
	// this is the slowest level, require some extra cpu power, depending on the config set scale.
    NotifyWithMaybeChangedKeys,
}

impl CFGCenterInner {
    pub fn get_namespace_scoped_cfg_center(
        &self,
        namespace: &str,
    ) -> Result<Arc<NamespaceScopedCFGCenter>> {
        let namespaces = self
            .namespaces
            .lock()
            .or(Err(CCLibError::NamespaceError("get lock error")))?;
        if let Some(t) = namespaces.get(namespace) {
            return Ok(t.clone());
        }
        return Err(CCLibError::NamespaceError("no namespace found"));
    }

    pub fn create_namespace_scoped_cfg_center(
        &self,
        namespace: &str,
        notify_level: UpdateNotifyLevel,
		callback: Option<Box<dyn Fn(&Differ)+ Send + Sync>>,
    ) -> Result<Arc<NamespaceScopedCFGCenter>> {
        if !namespace.starts_with("/") || !namespace.ends_with("/") {
            return Err(CCLibError::NamespaceError("namespace must starts and end with `/`"));
        }

		let cur_version = self.current_version.lock().or(Err(CCLibError::NamespaceError("get internal lock error")))?;

        let mem_store = Box::new(MemStorage::new(self.backend.as_ref(), namespace, &cur_version)?);

        let v = Arc::new(NamespaceScopedCFGCenter::new(
            namespace,
            mem_store,
			self.backend.clone(),
            notify_level,
			callback,
        ));

        let mut namespaces = self
            .namespaces
            .lock()
            .or(Err(CCLibError::NamespaceError("get lock error")))?;
        namespaces.insert(namespace.to_owned(), v.clone());
        Ok(v)
    }

    fn update_callback(&self, e: StorageChangeEvent) {
        let namespaces = match self.namespaces.lock() {
            Ok(t) => t,
            Err(_) => return,
        };

		let mut old_version = match self.current_version.lock() {
			Ok(t) => t,
            Err(_) => return,
		};


        for (ns, scoped_cfg_center) in &*namespaces {
            match scoped_cfg_center.notify_level {
                UpdateNotifyLevel::NotifyWithoutChangedKeysByGlobal => {
                    let new_mem_store = match MemStorage::new(self.backend.as_ref(), ns, &e.new_version) {
                        Ok(t) => Box::new(t),
                        Err(e) => {
                            print_error_with_switch!("error occured while loading changed configs in background, namespace = {}, {}", ns, e);
                            continue
                        }
                    };
					scoped_cfg_center.update_callback(new_mem_store, Vec::new());
				}
				UpdateNotifyLevel::NotifyWithoutChangedKeysInNamespace => {
					let changed_files = match self.backend.get_diff_list(&old_version, &e.new_version, &ns){
						Ok(t) => t,
						Err(_) => continue,
					};
					if changed_files.len() == 0 {
						continue
					}

                    let new_mem_store = match MemStorage::new(self.backend.as_ref(), &ns, &e.new_version) {
                        Ok(t) => Box::new(t),
                        Err(e) => {
                            print_error_with_switch!("error occured while loading changed configs in background, namespace = {}, {}", ns, e);
                            continue
                        }
                    };
					scoped_cfg_center.update_callback(new_mem_store, changed_files);
				}
                UpdateNotifyLevel::NotifyWithMaybeChangedKeys => {
					let changed_files = match self.backend.get_diff_list(&old_version, &e.new_version, &ns){
						Ok(t) => t,
						Err(_) => continue,
					};
					if changed_files.len() == 0 {
						continue
					}
					let new_mem_store = match MemStorage::new(self.backend.as_ref(), &ns, &e.new_version) {
                        Ok(t) => Box::new(t),
                        Err(e) => {
                            print_error_with_switch!("error occured while loading changed configs in background, namespace = {}, {}", ns, e);
                            continue
                        }
                    };
				}
                _ => {}
            };
        }

		*old_version = e.new_version;
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
    pub fn new(backend: Box<dyn storage_backends::StorageBackend + Send + Sync>) -> Result<Self> {

		let version = backend.get_current_version().or(Err(CCLibError::NamespaceError("can not get newest config version")))?;

        let inner = Arc::new(CFGCenterInner {
            backend: Arc::from(backend),
            namespaces: Mutex::new(HashMap::new()),
			current_version: Mutex::new(version),
        });

        let inner_for_capture = inner.clone();
        inner
            .backend
            .set_update_cb(Box::new(move |x| inner_for_capture.update_callback(x)))?;

        let t = CFGCenter(inner);
        Ok(t)
    }

    pub fn get_namespace_scoped_cfg_center(
        &self,
        namespace: &str,
    ) -> Result<Arc<NamespaceScopedCFGCenter>> {
        return self.0.get_namespace_scoped_cfg_center(namespace);
    }

    pub fn create_namespace_scoped_cfg_center(
        &self,
        namespace: &str,
        notify_level: UpdateNotifyLevel,
		callback: Option<Box<dyn Fn(&Differ) + Send + Sync>>,
    ) -> Result<Arc<NamespaceScopedCFGCenter>> {
        return self
            .0
            .create_namespace_scoped_cfg_center(namespace, notify_level, callback);
    }
}
