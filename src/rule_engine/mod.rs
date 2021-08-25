use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleMeta {
    pub desc: String,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct RuleSpec {
    pub rule: Condition,
}
#[derive(Debug, PartialEq)]
pub struct Rule {
    pub meta: RuleMeta,
    pub spec: RuleSpec,
}

#[derive(Debug, PartialEq)]
pub enum LeafOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Exist, // if value indicated by lhs exist in match context
    InList,
}

#[derive(Debug, PartialEq)]
pub enum Condition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Leaf {
        lhs: String,
        op: LeafOperator,
        rhs: Option<Value>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Null,
}

type MatchContext = HashMap<String, Value>;

impl Condition {
    pub fn eval(&self, ctx: &MatchContext) -> bool {
        match self {
            Condition::And(conds) => {
                for cond in conds {
                    if cond.eval(ctx) == false {
                        return false;
                    }
                }
                return true;
            }
            Condition::Or(conds) => {
                for cond in conds {
                    if cond.eval(ctx) == true {
                        return true;
                    }
                }
                return false;
            }
            Condition::Not(cond) => return !cond.eval(ctx),
            Condition::Leaf { lhs, op, rhs } => {
                return self.eval_leaf(ctx, lhs, op, rhs);
            }
        }
    }

    #[inline(always)]
    fn eval_leaf(
        &self,
        ctx: &MatchContext,
        lhs: &String,
        op: &LeafOperator,
        rhs: &Option<Value>,
    ) -> bool {
        let lhs_value = if let Some(v) = ctx.get(lhs) {
            v
        } else {
            return false;
        };

        if let Some(rhs_value) = rhs {
            // this branch is for binary operators
            match op {
                LeafOperator::Eq => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv == rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv == rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv == rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv == rv,
                    (Value::Null, Value::Null) => return true,
                    _ => return false,
                },
                LeafOperator::Ne => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv != rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv != rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv != rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv != rv,
                    (Value::Null, Value::Null) => return false,
                    _ => return true,
                },
                LeafOperator::Gt => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv > rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv > rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv > rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv > rv,
                    _ => return false,
                },
                LeafOperator::Gte => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv >= rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv >= rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv >= rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv >= rv,
                    _ => return false,
                },
                LeafOperator::Lt => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv < rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv < rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv < rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv < rv,
                    _ => return false,
                },
                LeafOperator::Lte => match (lhs_value, rhs_value) {
                    (Value::Str(lv), Value::Str(rv)) => return lv <= rv,
                    (Value::Int(lv), Value::Int(rv)) => return lv <= rv,
                    (Value::Float(lv), Value::Float(rv)) => return lv <= rv,
                    (Value::Bool(lv), Value::Bool(rv)) => return lv <= rv,
                    _ => return false,
                },
                LeafOperator::InList => {
                    if let Value::List(rv_list) = rhs_value {
                        return rv_list.contains(lhs_value);
                    } else {
                        return false;
                    }
                }
                LeafOperator::Exist => {
                    return true; // the false way is handled at the begining of this function
                }
            }
        } else {
            // this branch is for unary operators
            match op {
                LeafOperator::Exist => {
                    return true; // the false way is handled at the begining of this function
                }
                _ => {
                    panic!("should not reach here")
                }
            }
        }
    }
}

#[test]
fn test_leaf_eval() {
    let mut ctx = MatchContext::new();
    ctx.insert("str".into(), Value::Str("str_value".into()));
    ctx.insert("int".into(), Value::Int(314));
    ctx.insert("float".into(), Value::Float(3.14));

    let cond = Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::Eq,
        rhs: Some(Value::Str("str_value".to_string())),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "int".to_string(),
        op: LeafOperator::Eq,
        rhs: Some(Value::Int(314)),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "float".to_string(),
        op: LeafOperator::Eq,
        rhs: Some(Value::Float(3.14)),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::InList,
        rhs: Some(Value::List(vec![
            Value::Str("aaa".into()),
            Value::Str("bbb".into()),
            Value::Str("str_value".into()),
        ])),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "int".to_string(),
        op: LeafOperator::InList,
        rhs: Some(Value::List(vec![
            Value::Int(123),
            Value::Int(314),
            Value::Int(789),
        ])),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "float".to_string(),
        op: LeafOperator::InList,
        rhs: Some(Value::List(vec![Value::Float(1.23), Value::Float(3.14)])),
    };
    assert!(cond.eval(&ctx) == true);

    // although Exist is an unary operator, the rhs can exist, it's simply ignored
    let cond = Condition::Leaf {
        lhs: "float".to_string(),
        op: LeafOperator::Exist,
        rhs: Some(Value::List(vec![Value::Float(1.23), Value::Float(3.14)])),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "float".to_string(),
        op: LeafOperator::Exist,
        rhs: None,
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::Gt,
        rhs: Some(Value::Str("str".into())),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "int".to_string(),
        op: LeafOperator::Gte,
        rhs: Some(Value::Int(314)),
    };
    assert!(cond.eval(&ctx) == true);

    let cond = Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::Lt,
        rhs: Some(Value::Str("str".into())),
    };
    assert!(cond.eval(&ctx) == false);

    let cond = Condition::Leaf {
        lhs: "int".to_string(),
        op: LeafOperator::Lte,
        rhs: Some(Value::Int(314)),
    };
    assert!(cond.eval(&ctx) == true);
}

#[test]
fn test_logic_op() {
    let mut ctx = MatchContext::new();
    ctx.insert("str".into(), Value::Str("str_value".into()));
    ctx.insert("int".into(), Value::Int(314));
    ctx.insert("float".into(), Value::Float(3.14));

    let cond = Condition::Not(Box::new(Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::Eq,
        rhs: Some(Value::Str("str_value".to_string())),
    }));
    assert!(cond.eval(&ctx) == false);

    let cond = Condition::And(vec![
        Condition::Leaf {
            lhs: "str".to_string(),
            op: LeafOperator::Eq,
            rhs: Some(Value::Str("str_value".to_string())),
        },
        Condition::Leaf {
            lhs: "str".to_string(),
            op: LeafOperator::Eq,
            rhs: Some(Value::Str("str_value_1".to_string())),
        },
    ]);
    assert!(cond.eval(&ctx) == false);

    let cond = Condition::Or(vec![
        Condition::Leaf {
            lhs: "str".to_string(),
            op: LeafOperator::Eq,
            rhs: Some(Value::Str("str_value".to_string())),
        },
        Condition::Leaf {
            lhs: "str".to_string(),
            op: LeafOperator::Eq,
            rhs: Some(Value::Str("str_value_1".to_string())),
        },
    ]);
    assert!(cond.eval(&ctx) == true);
}
