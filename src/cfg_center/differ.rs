use crate::{error::DifferError, rule_engine::MatchContext, storage_backends::StorageBackend};

use super::{cfg_center::{UpdateNotifyLevel, ViewMode}, mem_store::MemStorage, querier::{CFGResult, Querier}};

type Result<T> = std::result::Result<T, DifferError>;

pub struct Differ<'a> {
	notify_level: UpdateNotifyLevel,
	old_mem_store: &'a MemStorage,
	new_mem_store: &'a MemStorage,
	changed_files: &'a Vec<String>, 
	backend: &'a dyn StorageBackend, 
}

impl <'a> Differ<'a> {

	pub (crate) fn new(notify_level: UpdateNotifyLevel, old_mem_store: &'a MemStorage,new_mem_store: &'a MemStorage, changed_files: &'a Vec<String>, backend: &'a dyn StorageBackend) -> Self {
		return Differ{
			notify_level,
			old_mem_store,
			new_mem_store,
			changed_files,
			backend,
		}
	}

    pub fn diff_with_whoami() {

	}

	pub fn get_maybe_changed_keys(&self) -> Vec<String>{
		Vec::new()
	}

    pub fn get_from_old(
        &self,
        whoami: &MatchContext,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>>{
		Ok(Querier::get(self.old_mem_store, whoami, keys, view_mode, need_explain)?)
    }

    pub fn get_from_new(
        &self,
        whoami: &MatchContext,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>>{
		Ok(Querier::get(self.new_mem_store, whoami, keys, view_mode, need_explain)?)
    }
}
