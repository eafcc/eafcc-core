use crate::rule_engine::MatchContext;

use super::{cfg_center::ViewMode, cfgindex::IndexBuilder, mem_store::MemStorage, querier::{CFGResult, Querier}};





pub struct NamespaceScopedCFGCenter {
	namespace: String,
	current_memstore: Box<MemStorage>,
}

impl NamespaceScopedCFGCenter {
	pub (crate) fn new(namespace: &str, mem_store: Box<MemStorage>) -> Self {
		let ret = NamespaceScopedCFGCenter{
			namespace: namespace.to_owned(),
			current_memstore: mem_store,
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
        Querier::get(&self.current_memstore, whoami, keys, view_mode, need_explain)
    }
}