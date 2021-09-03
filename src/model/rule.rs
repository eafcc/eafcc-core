use super::RootCommon;
use crate::error::DataLoaderError;
use crate::parser::rule;

use crate::rule_engine::{Condition, LeafOperator, Rule, RuleMeta, RuleSpec, Value};

use serde_json;

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
			"rule": "str == \"123\" && int == \"-345\" || ( float == \"-1.234\" && ! ( str == \"123\" ) )"
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
                            rhs: Some(Value::Str("-345".into()),),
                        },
                    ],),
                    Condition::And(vec![
                        Condition::Leaf {
                            lhs: "float".into(),
                            op: LeafOperator::Eq,
                            rhs: Some(Value::Str("-1.234".into()),),
                        },
                        Condition::Not(Box::new(Condition::Leaf {
                            lhs: "str".into(),
                            op: LeafOperator::Eq,
                            rhs: Some(Value::Str("123".into(),),),
                        }),),
                    ],),
                ],),
            },
        }
    )
}
