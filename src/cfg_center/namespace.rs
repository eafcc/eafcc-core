use std::sync::RwLock;

use crate::rule_engine::MatchContext;

use super::{cfg_center::{UpdateNotifyLevel, ViewMode}, cfgindex::IndexBuilder, mem_store::MemStorage, querier::{CFGResult, Querier}};





pub struct NamespaceScopedCFGCenter {
	namespace: String,
	current_memstore: RwLock<Box<MemStorage>>,
	pub (crate) notify_level: UpdateNotifyLevel,
}

impl NamespaceScopedCFGCenter {
	pub (crate) fn new(namespace: &str, mem_store: Box<MemStorage>, notify_level: UpdateNotifyLevel) -> Self {
		let ret = NamespaceScopedCFGCenter{
			namespace: namespace.to_owned(),
			current_memstore: RwLock::new(mem_store),
			notify_level
		};

		ret
	}

	pub fn get_cfg(
        &self,
        whoami: &MatchContext,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>, String> {
		let current_memstore = self.current_memstore.read().map_err(|e|e.to_string())?;
        Querier::get(&current_memstore, whoami, keys, view_mode, need_explain)
    }

	pub (crate) fn update_callback(&self, new_mem_store: Box<MemStorage>, changes: Vec<String>) {
		match self.notify_level {
			UpdateNotifyLevel::NotifyWithoutChangedKeysByGlobal => {
				let mut current_memstore = match self.current_memstore.write(){
					Ok(t) => t,
					Err(_) => return,
				};
				*current_memstore = new_mem_store;
			},
			UpdateNotifyLevel::NotifyWithoutChangedKeysInNamespace => {
				let mut current_memstore = match self.current_memstore.write(){
					Ok(t) => t,
					Err(_) => return,
				};
				*current_memstore = new_mem_store;
			},
			UpdateNotifyLevel::NotifyWithMaybeChangedKeys => {},
			_ => return
		};	
	}
}