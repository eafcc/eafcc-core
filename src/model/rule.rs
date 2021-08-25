use super::RootCommon;
use crate::error::{DataLoaderError, DataMemStorageError};
use crate::parser::rule;
use crate::rule_engine::{Condition, LeafOperator, Value};
use crate::rule_engine::{Rule, RuleMeta, RuleSpec};
use serde_json;
use std::collections::BTreeMap;
use std::io::Seek;
use std::rc::Rc;
use std::sync::RwLock;

pub fn load_rule(rule_data: &[u8]) -> Result<Rule, DataLoaderError> {
    let root = serde_json::from_slice::<RootCommon>(rule_data)?;
    let meta = serde_json::from_value::<RuleMeta>(root.meta)?;

    if let Some(v) = root.spec.pointer("/rule") {
        if let Some(rule_str) = v.as_str() {
            let (left, cond) = rule::do_parse(rule_str)
                .map_err(|e| DataLoaderError::SpecParseError(e.to_string()))?;
            if left != "" {
                return Err(DataLoaderError::SpecParseError(format!(
                    "syntax error in rule, some part not recognised: {}",
                    left
                )));
            }
            let spec = RuleSpec { rule: cond };
            return Ok(Rule { meta, spec });
        }
    }
    return Err(DataLoaderError::SpecParseError(
        "no rule found or rule is not string".into(),
    ));
}




#[test]
fn test_load_rule() {
    let rule = r#"
	{
		"version": 1,
		"kind": "Rule",
		"meta": {
			"desc": "balabalabala",
			"tags": ["foo", "bar"]
		},
		"spec": {
			"rule": "str == \"123\" && int == -345 || ( float == -1.234 && ! ( str in [ \"123\" , \"456\"] ) )"
		}
		
	}
	"#;

    let r = load_rule(rule.as_bytes()).unwrap();
    assert_eq!(
        r,
        Rule {
            meta: RuleMeta {
                desc: "balabalabala".into(),
                tags: vec!["foo".into(), "bar".into()],
            },
            spec: RuleSpec {
                rule: Condition::Or(vec![
                    Condition::And(vec![
                        Condition::Leaf {
                            lhs: "str".into(),
                            op: LeafOperator::Eq,
                            rhs: Some(Value::Str("123".into(),),),
                        },
                        Condition::Leaf {
                            lhs: "int".into(),
                            op: LeafOperator::Eq,
                            rhs: Some(Value::Int(-345,),),
                        },
                    ],),
                    Condition::And(vec![
                        Condition::Leaf {
                            lhs: "float".into(),
                            op: LeafOperator::Eq,
                            rhs: Some(Value::Float(-1.234,),),
                        },
                        Condition::Not(Box::new(Condition::Leaf {
                            lhs: "str".into(),
                            op: LeafOperator::InList,
                            rhs: Some(Value::List(vec![
                                Value::Str("123".into(),),
                                Value::Str("456".into(),),
                            ],),),
                        }),),
                    ],),
                ],),
            },
        }
    )
}
