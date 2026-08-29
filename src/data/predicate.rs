//! Implementation of simple predicates to filter data, with a SQL like syntax.
use crate::data::{Card, Value, ValueRef};
use crate::error::PredicateError;
use gerana::{ParseError, Parser, Symbol, Terminal, Variable};
use itertools::Itertools;
use logos::{Lexer, Logos};
use serde_json::Number;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

/// Abstract representation of a predicate.
///
/// A predicate can be created directly or parsed from a string, using a SQL like syntax.
///
/// # Example
/// ```
/// use cartomata::data::{Predicate, Value, PredicatePath, PredicatePathPart};
///
/// let p = Predicate::from_string("power >= 100 AND name LIKE 'sample'").unwrap();
/// assert_eq!(
///     p,
///     Predicate::Ge(PredicatePath(vec![PredicatePathPart::Key("power".into())]), Value::Number(100.into()))
///         & Predicate::Like(PredicatePath(vec![PredicatePathPart::Key("name".into())]), Value::String("sample".into()))
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    Eq(PredicatePath, Value),
    Neq(PredicatePath, Value),
    In(PredicatePath, ValueSet),
    Like(PredicatePath, Value),
    Contains(PredicatePath, Value),
    Lt(PredicatePath, Value),
    Le(PredicatePath, Value),
    Gt(PredicatePath, Value),
    Ge(PredicatePath, Value),
}

#[derive(Debug, Clone)]
enum AnyValue {
    Set(ValueSet),
    Unit(Value),
}

/// Represents a set of values, used in predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueSet {
    Int(HashSet<i64>),
    Str(HashSet<String>),
}

impl Display for ValueSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(vs) => write!(f, "({})", vs.iter().join(", ")),
            Self::Str(vs) => write!(f, "({})", vs.iter().join(", ")),
        }
    }
}

impl From<HashSet<i64>> for ValueSet {
    fn from(value: HashSet<i64>) -> Self {
        Self::Int(value)
    }
}

impl From<HashSet<String>> for ValueSet {
    fn from(value: HashSet<String>) -> Self {
        Self::Str(value)
    }
}

impl From<HashSet<&'_ str>> for ValueSet {
    fn from(value: HashSet<&str>) -> Self {
        Self::Str(value.into_iter().map(String::from).collect())
    }
}

impl std::ops::BitAnd for Predicate {
    type Output = Predicate;
    fn bitand(self, rhs: Self) -> Self::Output {
        Predicate::And(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::BitOr for Predicate {
    type Output = Predicate;
    fn bitor(self, rhs: Self) -> Self::Output {
        Predicate::Or(Box::new(self), Box::new(rhs))
    }
}

impl std::ops::Not for Predicate {
    type Output = Predicate;
    fn not(self) -> Self::Output {
        Predicate::Not(Box::new(self))
    }
}

impl Predicate {
    /// Parses a string to a predicate.
    pub fn from_string(predicate: &str) -> Result<Self, ParseError<PredicateError>> {
        PredicateParser::new(predicate).parse()
    }

    /// Evaluates a predicate on an input card.
    pub fn eval(&self, card: &impl Card) -> bool {
        match self {
            Self::And(a, b) => a.eval(card) && b.eval(card),
            Self::Or(a, b) => a.eval(card) || b.eval(card),
            Self::Not(a) => !a.eval(card),
            Self::Eq(k, v) => card.get(k) == ValueRef::from(v),
            Self::Neq(k, v) => card.get(k) != ValueRef::from(v),
            Self::In(k, ValueSet::Int(vs)) => match card.get(k) {
                ValueRef::Number(x) => {
                    if let Some(x) = x.as_f64() {
                        x.fract() == 0.0 && vs.contains(&(x as i64))
                    } else {
                        x.as_i64().map(|v| vs.contains(&v)).unwrap_or(false)
                    }
                }
                ValueRef::String(x) => x.parse::<i64>().map(|x| vs.contains(&x)).unwrap_or(false),
                _ => false,
            },
            Self::In(k, ValueSet::Str(vs)) => match &card.get(k) {
                ValueRef::String(x) => vs.contains(*x),
                _ => false,
            },
            Self::Like(k, v) => match (card.get(k), v) {
                (ValueRef::String(a), Value::String(b)) => a.contains(b),
                _ => false,
            },
            Self::Lt(k, v) => card.get(k) < ValueRef::from(v),
            Self::Le(k, v) => card.get(k) <= ValueRef::from(v),
            Self::Gt(k, v) => card.get(k) > ValueRef::from(v),
            Self::Ge(k, v) => card.get(k) >= ValueRef::from(v),
            Self::Contains(k, v) => match card.get(k) {
                ValueRef::Array(a) => a.contains(&ValueRef::from(v)),
                ValueRef::ArrayRef(a) => a.contains(&v),
                ValueRef::Object(o) => {
                    if let Value::String(v) = v {
                        o.contains_key(v.as_str())
                    } else {
                        false
                    }
                }
                ValueRef::ObjectRef(o) => {
                    if let Value::String(v) = v {
                        o.contains_key(v)
                    } else {
                        false
                    }
                }
                _ => false,
            },
        }
    }
}

#[derive(Debug, Clone, Logos, Terminal)]
#[logos(skip r"[ \t\n\f]+")]
enum PredicateTerm {
    #[token("(")]
    ParenO,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(")")]
    ParenC,
    #[token("NOT", ignore(case))]
    Not,
    #[token("AND", ignore(case))]
    And,
    #[token("OR", ignore(case))]
    Or,
    #[gerana(desc = "a key")]
    #[regex("[a-z][a-z0-9-]*|`([^`]|``)*`", unescape_ident, ignore(case))]
    Key(String),
    #[gerana(desc = "an operator")]
    #[regex("=|!=|>|>=|<|<=|IN|LIKE|CONTAINS", Operator::new, priority = 3, ignore(case))]
    Op(Operator),
    #[gerana(desc = "a string value")]
    #[regex("'([^']|'')*'", unescape_str)]
    ValStr(String),
    #[gerana(desc = "an int value")]
    #[regex(r"[+-]?\d+", parse_int)]
    ValInt(i64),
    #[gerana(desc = "a float value")]
    #[regex(r"[+-]?(\d*\.\d+(e[+-]?\d+)?|\d+e[+-]?\d+)", parse_float, ignore(case))]
    ValFloat(f64),
    #[gerana(desc = "a bool value")]
    #[regex("true|false", parse_bool, ignore(case))]
    ValBool(bool),
    #[gerana(desc = "`NULL`")]
    #[regex("NULL|NIL", ignore(case))]
    ValNil,
}

impl std::fmt::Display for PredicateTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParenO => write!(f, "("),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::ParenC => write!(f, ")"),
            Self::Not => write!(f, "NOT"),
            Self::And => write!(f, "AND"),
            Self::Or => write!(f, "OR"),
            Self::Key(key) => write!(f, "key {key}"),
            Self::Op(op) => write!(f, "operator {op}"),
            Self::ValStr(v) => write!(f, "string {}", escape_str(&v)),
            Self::ValInt(v) => write!(f, "integer {v}"),
            Self::ValFloat(v) => write!(f, "number {v}"),
            Self::ValBool(v) => write!(f, "boolean {v}"),
            Self::ValNil => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredicatePath(pub Vec<PredicatePathPart>);

impl std::fmt::Display for PredicatePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            itertools::join(self.0.iter().map(|p| p.to_string()), ".")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredicatePathPart {
    Key(String),
    Index(i64),
}

impl std::fmt::Display for PredicatePathPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key(p) => write!(f, "{p}"),
            Self::Index(p) => write!(f, "{p}"),
        }
    }
}

impl From<String> for PredicatePathPart {
    fn from(value: String) -> Self {
        Self::Key(value)
    }
}

impl From<&str> for PredicatePathPart {
    fn from(value: &str) -> Self {
        Self::Key(value.to_string())
    }
}

impl From<i64> for PredicatePathPart {
    fn from(value: i64) -> Self {
        Self::Index(value)
    }
}

/// A trait for dynamic access of data structure fields.
///
/// This trait is used primarily by predicate evaluation, but can also be used by other parts of
/// the pipeline execution to work with arbitrary [Card] types.
pub trait Access {
    fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_>;
}

macro_rules! access_leaf {
    ($($V:ty)+) => {
        $(
            impl Access for $V {
                fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_> {
                    if parts.is_empty() {
                        ValueRef::from(self)
                    } else {
                        ValueRef::Null
                    }
                }
            }
        )*
    };
}

access_leaf!(i64 i32 i16 i8 u64 u32 u16 u8 f64 f32 bool String str Number Value);

impl<T: Access> Access for Option<T> {
    fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_> {
        self.as_ref().map(|x| x.access(parts)).unwrap_or_default()
    }
}

impl<T: Access> Access for Vec<T> {
    fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_> {
        match parts {
            [PredicatePathPart::Index(i), rest @ ..] => {
                let index = if *i >= 0 {
                    Some(*i as usize)
                } else {
                    if let Ok(n) = i64::try_from(self.len()) {
                        n.checked_add(*i).map(|i| i as usize)
                    } else {
                        return ValueRef::Null;
                    }
                };
                if let Some(index) = index {
                    self.get(index).map(|x| x.access(rest)).unwrap_or_default()
                } else {
                    ValueRef::Null
                }
            }
            _ => ValueRef::Null,
        }
    }
}

impl<T: Access> Access for HashMap<String, T> {
    fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_> {
        match parts {
            [PredicatePathPart::Key(key), rest @ ..] => {
                if let Some(value) = self.get(key.as_str()) {
                    value.access(rest)
                } else {
                    ValueRef::Null
                }
            }
            _ => ValueRef::Null,
        }
    }
}

impl<T: Access> Access for HashMap<&str, T> {
    fn access(&'_ self, parts: &[PredicatePathPart]) -> ValueRef<'_> {
        match parts {
            [PredicatePathPart::Key(key), rest @ ..] => {
                if let Some(value) = self.get(key.as_str()) {
                    value.access(rest)
                } else {
                    ValueRef::Null
                }
            }
            _ => ValueRef::Null,
        }
    }
}

#[derive(Debug, Clone)]
enum Operator {
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    Like,
    Contains,
}

impl Operator {
    fn new(lex: &mut Lexer<PredicateTerm>) -> Self {
        match lex.slice().to_uppercase().as_str() {
            "=" => Self::Eq,
            "!=" => Self::Neq,
            "<" => Self::Lt,
            "<=" => Self::Le,
            ">" => Self::Gt,
            ">=" => Self::Ge,
            "IN" => Self::In,
            "LIKE" => Self::Like,
            "CONTAINS" => Self::Contains,
            _ => unreachable!("invalid operator"),
        }
    }

    fn predicate(self, key: PredicatePath, val: AnyValue) -> Result<Predicate, PredicateError> {
        match (&self, val) {
            (Self::Eq, AnyValue::Unit(v)) => Ok(Predicate::Eq(key, v)),
            (Self::Neq, AnyValue::Unit(v)) => Ok(Predicate::Neq(key, v)),
            (Self::Lt, AnyValue::Unit(v)) => Ok(Predicate::Lt(key, v)),
            (Self::Le, AnyValue::Unit(v)) => Ok(Predicate::Le(key, v)),
            (Self::Gt, AnyValue::Unit(v)) => Ok(Predicate::Gt(key, v)),
            (Self::Ge, AnyValue::Unit(v)) => Ok(Predicate::Ge(key, v)),
            (Self::In, AnyValue::Set(v)) => Ok(Predicate::In(key, v)),
            (Self::Like, AnyValue::Unit(v)) => Ok(Predicate::Like(key, v)),
            (Self::In, AnyValue::Unit(v)) => Err(PredicateError::bad_operand(self, "a set", v)),
            (Self::Contains, AnyValue::Unit(v)) => Ok(Predicate::Contains(key, v)),
            (_, AnyValue::Set(v)) => Err(PredicateError::bad_operand(self, "a single value", v)),
        }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "="),
            Self::Neq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::Le => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::Ge => write!(f, ">="),
            Self::In => write!(f, "IN"),
            Self::Like => write!(f, "LIKE"),
            Self::Contains => write!(f, "CONTAINS"),
        }
    }
}

fn unescape_ident(lex: &Lexer<PredicateTerm>) -> String {
    let escaped = lex.slice().chars().next().is_some_and(|x| x == '`');
    if escaped {
        let span = lex.span();
        lex.source()[span.start + 1..span.end - 1].replace("``", "`")
    } else {
        lex.slice().to_string()
    }
}

fn unescape_str(lex: &Lexer<PredicateTerm>) -> String {
    let span = lex.span();
    lex.source()[span.start + 1..span.end - 1].replace("''", "'")
}

fn escape_str(s: &String) -> String {
    format!("'{}'", s.replace("'", "''"))
}

fn parse_int(lex: &Lexer<PredicateTerm>) -> i64 {
    lex.slice().parse().unwrap()
}

fn parse_float(lex: &Lexer<PredicateTerm>) -> f64 {
    lex.slice().parse().unwrap()
}

fn parse_bool(lex: &Lexer<PredicateTerm>) -> bool {
    lex.slice().to_lowercase().parse().unwrap()
}

#[derive(Debug, Parser)]
#[gerana(error = PredicateError)]
#[rule(Ex => Ex(p1) :Or E1(p2) { p1 | p2 } )]
#[rule(Ex => E1(p) { p } )]
#[rule(E1 => E1(p1) :And E2(p2) { p1 & p2 } )]
#[rule(E1 => E2(p) { p } )]
#[rule(E2 => :ParenO Ex(p) :ParenC { p } )]
#[rule(E2 => :Not E2(p) { !p } )]
#[rule(E2 => P(p) :Op(op) V(val) { op.predicate(p, val)? } )]
#[rule(P  => Pp(p) { p })]
#[rule(Pp => :Key(key) { PredicatePath(vec![key.into()]) })]
#[rule(Pp => Pp(mut p) :Dot :Key(key) { p.0.push(key.into()); p })]
#[rule(Pp => Pp(mut p) :Dot :ValInt(i) { p.0.push(i.into()); p })]
#[rule(V  => :ParenO S(s) :ParenC { AnyValue::Set(s) } )]
#[rule(S  => Si(mut s) :ValInt(v) { s.insert(v); ValueSet::Int(s) } )]
#[rule(S  => Ss(mut s) :ValStr(v) { s.insert(v); ValueSet::Str(s) } )]
#[rule(Si => Si(mut s) :ValInt(v) :Comma { s.insert(v); s } )]
#[rule(Si => { HashSet::new() } )]
#[rule(Ss => Ss(mut s) :ValStr(v) :Comma { s.insert(v); s } )]
#[rule(Ss => { HashSet::new() } )]
#[rule(V  => :ValInt(v) { AnyValue::Unit(Value::from(v)) } )]
#[rule(V  => :ValStr(v) { AnyValue::Unit(Value::from(v)) } )]
#[rule(V  => :ValFloat(v) { AnyValue::Unit(Value::from(v)) } )]
#[rule(V  => :ValBool(v) { AnyValue::Unit(Value::from(v)) } )]
#[rule(V  => :ValNil { AnyValue::Unit(Value::Null) } )]
struct PredicateParser<'src> {
    lexer: Lexer<'src, PredicateTerm>,
    symbol_stack: Vec<Symbol<PredicateVar, PredicateTerm>>,
    state_stack: Vec<usize>,
}

#[derive(Debug, Clone, Variable)]
enum PredicateVar {
    Ex(Predicate),
    E1(Predicate),
    E2(Predicate),
    P(PredicatePath),
    Pp(PredicatePath),
    V(AnyValue),
    S(ValueSet),
    Si(HashSet<i64>),
    Ss(HashSet<String>),
}
