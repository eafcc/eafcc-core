use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum LeafOperator {
    Eq,
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Str(String),
}

pub type MatchContext = HashMap<String, Value>;

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
                },
            }
        } else {
            // this branch is for unary operators
            match op {
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

    let cond = Condition::Leaf {
        lhs: "str".to_string(),
        op: LeafOperator::Eq,
        rhs: Some(Value::Str("str_value".to_string())),
    };
    assert!(cond.eval(&ctx) == true);
}

#[test]
fn test_logic_op() {
    let mut ctx = MatchContext::new();
    ctx.insert("str".into(), Value::Str("str_value".into()));

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
