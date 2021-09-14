use std::cmp::Ordering;
use std::sync::Arc;

use crate::cfg_center::mem_store::MemStorage;
use crate::error::QueryError;
use crate::rule_engine;

use super::ViewMode;
use super::cfgindex::{IdxLinkItem, KeyValuePair};
pub struct Querier {}

type Result<T> = std::result::Result<T, QueryError>;

pub struct CFGResult {
    pub reason: Option<Arc<IdxLinkItem>>,
    pub value: Arc<KeyValuePair>,
}

impl Querier {
    pub fn get(
        mem_store: &MemStorage,
        whoami: &rule_engine::MatchContext,
        keys: &Vec<&str>,
        view_mode: ViewMode,
        need_explain: bool,
    ) -> Result<Vec<CFGResult>> {
        let mut act_links = Vec::new();

        mem_store
            .indices
            .rule_stor
            .iter_related_rules(whoami, |rule| {
                if rule.rule.eval(whoami) {
					if let Some(links) = mem_store.indices.link_stor.get_link_by_rule_path(&rule.abs_path){
						for link in links{
							act_links.push(link.clone());

						}
					}
                }
            });

		if act_links.len() == 0{
			return Ok(Vec::new());
		}
        
        let ret = match view_mode {
            ViewMode::OverlaidView => fetch_res_by_overlaid_view(&*mem_store, keys, act_links, need_explain),
            ViewMode::AllLinkedResView => fetch_res_by_all_linked_res_view(&*mem_store, keys, act_links, need_explain),
        };
        return Ok(ret);
    }
}

fn fetch_res_by_overlaid_view(
    mem_store: &MemStorage,
    keys: &Vec<&str>,
    mut links: Vec<Arc<IdxLinkItem>>,
    need_explain: bool,
) -> Vec<CFGResult> {
    let mut ret_buf = Vec::with_capacity(keys.len());

    links.sort_unstable_by(|a, b| {
        // safety: infinate value is filtered out when loading links from storage
        if a.pri > b.pri {
            return Ordering::Less;
        } else if a.pri == b.pri {
            // if a is neg, no matter what b is, we can put a before b, if b is also neg, the order between a and b does not matter anymore
            return if a.is_neg {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        } else {
            return Ordering::Greater;
        }
    });

    for key in keys {
		'next_key: 
		for link in &links {
			if let Some(res) = mem_store.indices.res_stor.get_res_by_path(&link.abs_res_path) {
				for kv_item in &res.data {
					if kv_item.key == *key {
                        if !link.is_neg {
                            let reason = if need_explain {
                                Some(link.clone())
                            } else {
                                None
                            };

                            ret_buf.push(CFGResult {
                                reason,
                                value: kv_item.clone(),
                            });
                        }
                        break 'next_key;
                    }
				}
			}
		}
    }
    return ret_buf;
}

fn fetch_res_by_all_linked_res_view(
	mem_store: &MemStorage,
    keys: &Vec<&str>,
    mut links: Vec<Arc<IdxLinkItem>>,
    need_explain: bool,
) -> Vec<CFGResult> {
    let mut ret_buf = Vec::with_capacity(keys.len());



	for key in keys {

		for link in &links {
			if let Some(res) = mem_store.indices.res_stor.get_res_by_path(&link.abs_res_path) {
				for kv_item in &res.data {
					if kv_item.key == *key {
                        let reason = if need_explain {
                            Some(link.clone())
                        } else {
                            None
                        };
                        unsafe {
                            // safety: we can ensure only one of the closure will be called, so ret_buf can be mut borrowed in to closures
                            let mut t =
                                &mut *(&ret_buf as *const Vec<CFGResult> as *mut Vec<CFGResult>);
                            t.push(CFGResult {
                                reason,
                                value: kv_item.clone(),
                            });
                        }
                    }
				}
			}
		}
    }


    
    return ret_buf;
}
