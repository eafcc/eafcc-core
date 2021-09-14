use std::{collections::HashMap, sync::Arc};

use crate::error::MemoryIndexError;
use crate::rule_engine::{Condition, MatchContext};

use crate::model;
use crate::storage_backends::StorageBackend;

type Result<T> = std::result::Result<T, MemoryIndexError>;

pub struct CFGIndex {
    pub rule_stor: RuleIndex,
    pub res_stor: ResIndex,
    pub link_stor: LinkIndex,
}



pub struct IndexBuilder {}

impl IndexBuilder {
    pub fn new() -> Self {
        return Self {};
    }

    fn load_rule(
        backend: &dyn StorageBackend,
        namespace: &str,
        index: &mut CFGIndex,
        version: &str,
    ) -> Result<()> {
        let mut nodes_to_visit: Vec<String> = Vec::with_capacity(32);
        nodes_to_visit.push("/rules".to_string() + namespace);
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in backend.list_dir(version, &parent_node)?{
                let full_path = parent_node.to_owned() + &cur_node.name;
                if cur_node.name.ends_with("/") {
                    nodes_to_visit.push(full_path);
                    continue;
                }
                let rule_raw_data = backend.get_obj_by_hash(&cur_node.hash)?;
                let rule_obj = model::rule::Rule::load_from_slice(&rule_raw_data)?;
				let str_skio_internal_prefix = &full_path[6..];
                index.rule_stor.add_rule(str_skio_internal_prefix, &rule_obj);
            }
        }
        return Ok(());
    }

    fn load_link(
        backend: &dyn StorageBackend,
        namespace: &str,
        index: &mut CFGIndex,
        version: &str,
    ) -> Result<()> {
        let mut nodes_to_visit: Vec<String> = Vec::with_capacity(32);
        nodes_to_visit.push("/links".to_string() + namespace);
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in backend.list_dir(version, &parent_node)? {
                let full_path = parent_node.to_owned() + &cur_node.name;
                if cur_node.name.ends_with("/") {
                    nodes_to_visit.push(full_path);
                    continue;
                }
                let link_raw_data = backend.get_obj_by_hash(&cur_node.hash)?;
                let link_obj = model::link::Link::load_from_slice(&link_raw_data)?;
				let str_skio_internal_prefix = &full_path[6..];
                index.link_stor.add_link(str_skio_internal_prefix, &link_obj);
            }
        }
        return Ok(());
    }

    fn load_res(
        backend: &dyn StorageBackend,
        namespace: &str,
        index: &mut CFGIndex,
        version: &str,
    ) -> Result<()> {
        let mut nodes_to_visit: Vec<String> = Vec::with_capacity(32);
        nodes_to_visit.push("/reses".to_string() + namespace);
        while let Some(ref parent_node) = nodes_to_visit.pop() {
            for cur_node in backend.list_dir(version, &parent_node)? {
                let full_path = parent_node.to_owned() + &cur_node.name;
                if cur_node.name.ends_with("/") {
                    nodes_to_visit.push(full_path);
                    continue;
                }
                let res_raw_data = backend.get_obj_by_hash(&cur_node.hash)?;
                let res_group_obj = model::res::Res::load_from_slice(&res_raw_data)?;
				let str_skio_internal_prefix = &full_path[6..];
                index.res_stor.add_res(str_skio_internal_prefix, res_group_obj);
            }
        }
        return Ok(());
    }

    pub fn load(backend: &dyn StorageBackend, namespace:&str, version: &str) -> Result<CFGIndex> {
        let mut cfg_index = CFGIndex {
            rule_stor: RuleIndex::new(),
            res_stor: ResIndex::new(),
            link_stor: LinkIndex::new(),
        };

        Self::load_rule(backend, namespace, &mut cfg_index, version)?;
        Self::load_link(backend, namespace, &mut cfg_index, version)?;
        Self::load_res(backend, namespace, &mut cfg_index, version)?;
		Ok(cfg_index)
    }
}

#[derive(Debug)]

pub struct IdxRuleItem {
    pub rule: Condition,
    pub abs_path: String,
}

pub struct RuleIndex {
    storage: HashMap<String, Arc<IdxRuleItem>>,
}

impl RuleIndex {
    pub fn new() -> Self {
        let storage = HashMap::new();
        return Self { storage };
    }

    pub fn add_rule(
        &mut self,
        abs_path: &str,
        rule: &model::rule::Rule,
    ) {
        self.storage.insert(
            abs_path.to_owned(),
            Arc::new(IdxRuleItem {
                rule: rule.spec.rule.clone(),
                abs_path: abs_path.to_owned(),
            }),
        );
    }

    pub fn iter_related_rules(&self, whoami: &MatchContext, mut cb: impl FnMut(&IdxRuleItem)) {
        for (_, v) in &self.storage {
            cb(v);
        }
    }
}

pub struct IdxLinkItem {
    pub pri: f32,
    pub is_neg: bool,

    pub rule_path: String,
    pub abs_res_path: String, // the abs path in filesystem, eg, if the origin res is selected by tag, then this field should not be tag, it must be a real link object file
    pub link_path: Arc<String>,
}

pub struct LinkIndex {
    idx_rule_to_res: HashMap<String, Vec<Arc<IdxLinkItem>>>,
}

impl LinkIndex {
    pub fn new() -> Self {
        return Self {
            idx_rule_to_res: HashMap::new(),
        };
    }

    pub fn add_link(
        &mut self,
        link_path: &str,
        link: &model::link::Link,
    ) {
        let arc_link_path = Arc::new(link_path.to_owned());

        let v: Vec<_> = link
            .spec
            .reses
            .iter()
            .map(|res_path| {
                Arc::new(IdxLinkItem {
                    pri: link.spec.pri,
                    is_neg: link.spec.is_neg,
                    rule_path: link.spec.rule.to_owned(),
                    abs_res_path: remove_path_type_prefix(res_path).to_owned(),
                    link_path: arc_link_path.clone(),
                })
            })
            .collect();


        let rule_abs_path = remove_path_type_prefix(&link.spec.rule).to_owned();

        self.idx_rule_to_res
            .entry(rule_abs_path)
            .or_default()
            .extend(v);
    }

    pub fn get_link_by_rule_path(&self, rule_path: &str) -> Option<&Vec<Arc<IdxLinkItem>>> {
        self.idx_rule_to_res.get(rule_path)
    }
}

pub struct KeyValuePair {
    pub content_type: String,
    pub key: String,
    pub value: String,
}

pub struct Resource {
    pub data: Vec<Arc<KeyValuePair>>,
    pub(crate) res_path: Arc<String>,
}

pub struct ResIndex {
    storage: HashMap<String, Resource>,
}

impl ResIndex {
    pub fn new() -> Self {
        return Self {
            storage: HashMap::new(),
        };
    }

    pub fn add_res(
        &mut self,
        res_path: &str,
        res: model::res::Res,
    ) {
        let data: Vec<_> = res
            .spec
            .0
            .into_iter()
            .map(|spec| {
                Arc::new(KeyValuePair {
                    content_type: spec.content_type,
                    key: spec.key,
                    value: spec.data,
                })
            })
            .collect();
        self.storage.insert(
            res_path.to_owned(),
            Resource {
                data,
                res_path: Arc::new(res_path.to_owned()),
            },
        );
    }

    pub fn get_res_by_path(&self, res_path: &str) -> Option<&Resource> {
        self.storage.get(res_path)
    }
}

fn remove_path_type_prefix(i: &str) -> &str {
    // remove `path:`
    return &i[5..];
}
