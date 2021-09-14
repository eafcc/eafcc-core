use std::sync::{Arc, RwLock};

use crate::{error::QueryError, rule_engine::MatchContext, storage_backends};

use super::{
    cfg_center::{UpdateNotifyLevel, ViewMode},
    cfgindex::IndexBuilder,
    differ::Differ,
    mem_store::MemStorage,
    querier::{CFGResult, Querier},
};

type Result<T> = std::result::Result<T, QueryError>;

#[repr(u32)]
pub enum UpdateInfoEventType {
    KeyCreate = 1,
    KeyModify = 2,
    KeyDelete = 3,
    KeyNotSure = 4,
}

pub struct UpdateEventItem {
    pub event_type: UpdateInfoEventType,
    pub key: String,
}

pub struct NamespaceScopedCFGCenter {
    pub(crate) namespace: String,
    pub(crate) current_memstore: RwLock<Box<MemStorage>>,
    pub(crate) notify_level: UpdateNotifyLevel,
    pub(crate) callback: Option<Box<dyn Fn(&Differ) + Send + Sync>>,
    backend: Arc<dyn storage_backends::StorageBackend + Send + Sync>,
}

impl NamespaceScopedCFGCenter {
    pub(crate) fn new(
        namespace: &str,
        mem_store: Box<MemStorage>,
        backend: Arc<dyn storage_backends::StorageBackend + Send + Sync>,
        notify_level: UpdateNotifyLevel,
        callback: Option<Box<dyn Fn(&Differ) + Send + Sync>>,
    ) -> Self {
        let ret = NamespaceScopedCFGCenter {
            namespace: namespace.to_owned(),
            current_memstore: RwLock::new(mem_store),
            notify_level,
            callback,
            backend,
        };

        ret
    }

    pub fn get_cfg(
        &self,
        whoami: &MatchContext,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>> {
        let current_memstore = self.current_memstore.read().or(Err(QueryError::GetLockError))?;
        Querier::get(&current_memstore, whoami, keys, view_mode, need_explain)
    }

    pub(crate) fn update_callback(&self, new_mem_store: Box<MemStorage>, changes: Vec<String>) {
        match self.notify_level {
            UpdateNotifyLevel::NotifyWithoutChangedKeysByGlobal => {
                if let Some(cb) = &self.callback {
                    let old_memstore = match self.current_memstore.read() {
                        Ok(t) => t,
                        Err(_) => return,
                    };
                    let differ = Differ::new(
                        self.notify_level,
                        &old_memstore,
                        &new_mem_store,
                        &changes,
                        self.backend.as_ref(),
                    );
                    cb(&differ);
                }
            }
            UpdateNotifyLevel::NotifyWithoutChangedKeysInNamespace => {}
            UpdateNotifyLevel::NotifyWithMaybeChangedKeys => {}
            _ => return,
        };

        let mut current_memstore = match self.current_memstore.write() {
            Ok(t) => t,
            Err(_) => return,
        };
        *current_memstore = new_mem_store;
    }
}
