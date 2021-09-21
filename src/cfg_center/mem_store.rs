use crate::{cfg_center::cfgindex, error::MemoryIndexError, storage_backends::{StorageBackend, VersionItem}};

use super::cfgindex::IndexBuilder;

type Result<T> = std::result::Result<T, MemoryIndexError>;

pub struct MemStorage {
	pub version: VersionItem,
    // mem_cache: TODO, now index save all data in itself, but it should only store index, other data should be accessed from backend, but can use this cache to speed up
    pub indices: cfgindex::CFGIndex,
}

impl MemStorage {
    pub fn new(backend: &dyn StorageBackend, namespace: &str, version: &VersionItem) -> Result<Self> {
        let idx = IndexBuilder::load(backend, namespace, version)?;
        return Ok(MemStorage {version: version.to_owned(), indices: idx });
    }
}
