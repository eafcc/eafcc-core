

use nom::branch::alt;
use nom::bytes::complete::{escaped, tag, take_till1, take_while_m_n};
use nom::character::complete::{alpha1, alphanumeric1, multispace0};
use nom::combinator::{map, map_res, peek, recognize};
use nom::{IResult, error::{context, ParseError}, multi::many0, sequence::{delimited, pair, preceded, tuple}};

use unescape::unescape;

use crate::rule_engine::{Condition, LeafOperator, Value};

// reerence:
// https://github.com/balajisivaraman/basic_calculator_rs/blob/master/src/parser.rs
// https://zhuanlan.zhihu.com/p/146455601   (https://link.zhihu.com/?target=https%3A//github.com/PrivateRookie/jsonparse)

// priority(from higher to lower):
// ()
// !
// ==, >=, >, <=, <, !=
// &&
// ||

fn leaf_binary_op(i: &str) -> IResult<&str, LeafOperator> {
    let (i, op_str) = tag("==")(i)?;

    let op = match op_str {
        "==" => LeafOperator::Eq,
        _ => {
            panic!()
        }
    };

    return Ok((i, op));
}


pub fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}


fn rhs_literal(i: &str) -> IResult<&str, Value> {
    map_res(string, unescape_str)(i)
}

fn unescape_str(i: &str) -> Result<Value, String> {
	if let Some(t) =  unescape(i) {
        return Ok(Value::Str(t))
    } else {
        return Err("can not parse enscaped string".into())
    }
}


fn leaf_expr(i: &str) -> IResult<&str, Condition> {
    let (i,(ident, op, val)) = tuple((
		ws(identifier),
		ws(leaf_binary_op),
        ws(rhs_literal),
	))(i)?;

    let t = Condition::Leaf{
        lhs: String::from(ident),
        op,
        rhs: Some(val),
    };
    return Ok((i, t))

}

fn leaf_expr_or_paren(i: &str) -> IResult<&str, Condition> {
    alt((
        leaf_expr,
        delimited(ws(tag("(")), expr_no_paren, ws(tag(")"))),
    ))(i)
}


fn expr_not(i: &str) -> IResult<&str, Condition> {
    alt((
        map(pair(tag("!"), leaf_expr_or_paren), |(_, cond)|{Condition::Not(Box::new(cond))}),
        leaf_expr_or_paren,
    ))(i)
}

fn expr_and(i: &str) -> IResult<&str, Condition> {
    let (i, (first, more)) = tuple((
        ws(expr_not),
        many0(preceded(ws(tag("&&")), ws(expr_not)))
    ))(i)?;

    if more.len() == 0 {
        return Ok((i, first))
    } else {
        let mut children = Vec::with_capacity(1+more.len());
        children.push(first);
        children.extend(more);
        let ret = Condition::And(children);
        return Ok((i, ret))
    }
}

fn expr_or(i: &str) -> IResult<&str, Condition> {
    let (i, (first, more)) = tuple((
        ws(expr_and),
        many0(preceded(ws(tag("||")), ws(expr_and)))
    ))(i)?;

    if more.len() == 0 {
        return Ok((i, first))
    } else {
        let mut children = Vec::with_capacity(1+more.len());
        children.push(first);
        children.extend(more);
        let ret = Condition::Or(children);
        return Ok((i, ret))
    }
}

fn expr_no_paren(i: &str) -> IResult<&str, Condition> { 
    expr_or(i)
}



fn expr(i: &str) -> IResult<&str, Condition> { 
    expr_or(i)
}

pub fn do_parse(i: &str) -> IResult<&str, Condition> {
    expr(i)
}

fn string(i: &str) -> IResult<&str, &str> {
    context(
        "string",
        alt((tag("\"\""), delimited(tag("\""), parse_str, tag("\"")))),
    )(i)
}

fn parse_str(i: &str) -> IResult<&str, &str> {
    escaped(normal, '\\', escapable)(i)
}

fn normal(i: &str) -> IResult<&str, &str> {
    take_till1(|c: char| c == '\\' || c == '"' || c.is_ascii_control())(i)
}

fn escapable(i: &str) -> IResult<&str, &str> {
    context(
        "escaped",
        alt((
            tag("\""),
            tag("\\"),
            tag("/"),
            tag("b"),
            tag("f"),
            tag("n"),
            tag("r"),
            tag("t"),
            parse_hex,
        )),
    )(i)
}

fn parse_hex(i: &str) -> IResult<&str, &str> {
    context(
        "hex string",
        preceded(
            peek(tag("u")),
            take_while_m_n(5, 5, |c: char| c.is_ascii_hexdigit() || c == 'u'),
        ),
    )(i)
}

#[test]
fn rhs_literal_test() {
    assert_eq!(rhs_literal("\"3.0\""), Ok(("", Value::Str("3.0".into()))));
	assert_eq!(rhs_literal("\"3.0\\n\""), Ok(("", Value::Str("3.0\n".into()))));
}

#[test]
fn leaf_expr_test() {
    assert_eq!(
        leaf_expr("a == \"4\""),
        Ok((
            "",
            Condition::Leaf{
                lhs: "a".into(),
                op: LeafOperator::Eq,
                rhs: Some(Value::Str("4".into())),
            },
        ))
    )
    
}

#[test]
fn expr_not_test() {
    assert_eq!(
        expr_or("!a==\"1\""),
        Ok((
            "",
            Condition::Not(Box::new(
                Condition::Leaf{
                    lhs: "a".into(),
                    op: LeafOperator::Eq,
                    rhs: Some(Value::Str("1".into())),
                }
            )),
        ))
    );
}


#[test]
fn expr_or_test() {
    println!("{:#?}", expr_or("a==1 && b==2 && c==3 || d==4 && e==5"));
}
#[test]
fn expr_test() {

    println!("{:#?}", expr("b==2&&(c==3||d==4)&&e==5"));
}
