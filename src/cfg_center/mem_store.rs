use crate::{cfg_center::cfgindex, storage_backends::StorageBackend};

use super::cfgindex::IndexBuilder;
pub struct MemStorage {
    // mem_cache: TODO, now index save all data in itself, but it should only store index, other data should be accessed from backend, but can use this cache to speed up
    pub indices: cfgindex::CFGIndex,
}

impl MemStorage {
    pub fn new(backend: &dyn StorageBackend, namespace: &str, version: &str) -> Self {
        let idx = IndexBuilder::load(backend, namespace, version);
        return MemStorage { indices: idx };
    }
}
