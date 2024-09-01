#![allow(dead_code)]

use crate::data::{Card, Value};
use crate::error::{Error, Result};

use itertools::Itertools;
use logos::{Lexer, Logos};
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Predicate {
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    Eq(String, Value),
    Neq(String, Value),
    In(String, SetValue),
    Like(String, Value),
    Lt(String, Value),
    Le(String, Value),
    Gt(String, Value),
    Ge(String, Value),
}

#[derive(Debug, Clone)]
enum AnyValue {
    Set(SetValue),
    Unit(Value),
}

#[derive(Debug, Clone)]
pub enum SetValue {
    IntSet(HashSet<i64>),
    StrSet(HashSet<String>),
}

impl Display for SetValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IntSet(vs) => write!(f, "({})", vs.iter().join(", ")),
            Self::StrSet(vs) => write!(f, "({})", vs.iter().join(", ")),
        }
    }
}

impl From<HashSet<i64>> for SetValue {
    fn from(value: HashSet<i64>) -> Self {
        Self::IntSet(value)
    }
}

impl From<HashSet<String>> for SetValue {
    fn from(value: HashSet<String>) -> Self {
        Self::StrSet(value)
    }
}

impl From<HashSet<&'_ str>> for SetValue {
    fn from(value: HashSet<&str>) -> Self {
        Self::StrSet(value.into_iter().map(String::from).collect())
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
    pub fn from_string(predicate: &str) -> Result<Self> {
        Parser::new(predicate).parse()
    }

    pub fn eval(&self, card: &impl Card) -> bool {
        match self {
            Self::And(a, b) => a.eval(card) && b.eval(card),
            Self::Or(a, b) => a.eval(card) || b.eval(card),
            Self::Not(a) => !a.eval(card),
            Self::Eq(k, v) => &card.get(k) == v,
            Self::Neq(k, v) => &card.get(k) != v,
            Self::In(k, SetValue::IntSet(vs)) => match &card.get(k) {
                Value::Int(x) => vs.contains(x),
                Value::Float(x) => x.fract() == 0.0 && vs.contains(&(*x as i64)),
                Value::Str(x) => x.parse::<i64>().map(|x| vs.contains(&x)).unwrap_or(false),
                _ => false,
            },
            Self::In(k, SetValue::StrSet(vs)) => match &card.get(k) {
                Value::Str(x) => vs.contains(x),
                _ => false,
            },
            Self::Like(k, v) => card.get(k).to_string().contains(&v.to_string()),
            Self::Lt(k, v) => &card.get(k) < v,
            Self::Le(k, v) => &card.get(k) <= v,
            Self::Gt(k, v) => &card.get(k) > v,
            Self::Ge(k, v) => &card.get(k) >= v,
        }
    }
}

#[derive(Debug, Clone, Logos)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[token("(")]
    ParenO,
    #[token(",")]
    Comma,
    #[token(")")]
    ParenC,
    #[token("NOT", ignore(case))]
    Not,
    #[token("AND", ignore(case))]
    And,
    #[token("OR", ignore(case))]
    Or,
    #[regex("[a-z][a-z0-9-]*|`([^`]|``)*`", unescape_ident, ignore(case))]
    Key(String),
    #[regex("=|!=|>|>=|<|<=|IN|LIKE", Operator::new, priority = 3, ignore(case))]
    Op(Operator),
    #[regex("'([^']|'')*'", unescape_str)]
    ValStr(String),
    #[regex(r"[+-]?\d+", parse_int)]
    ValInt(i64),
    #[regex(r"[+-]?(\d*\.\d+(e[+-]?\d+)?|\d+e[+-]?\d+)", parse_float, ignore(case))]
    ValFloat(f64),
    #[regex("true|false", parse_bool, ignore(case))]
    ValBool(bool),
    #[regex("NULL|NIL", ignore(case))]
    ValNil,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParenO => write!(f, "("),
            Self::Comma => write!(f, ","),
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
}

impl Operator {
    fn new(lex: &mut Lexer<Token>) -> Self {
        match lex.slice().to_uppercase().as_str() {
            "=" => Self::Eq,
            "!=" => Self::Neq,
            "<" => Self::Lt,
            "<=" => Self::Le,
            ">" => Self::Gt,
            ">=" => Self::Ge,
            "IN" => Self::In,
            "LIKE" => Self::Like,
            _ => unreachable!("invalid operator"),
        }
    }

    fn predicate(self, key: String, val: AnyValue) -> Result<Predicate> {
        match (&self, val) {
            (Self::Eq, AnyValue::Unit(v)) => Ok(Predicate::Eq(key, v)),
            (Self::Neq, AnyValue::Unit(v)) => Ok(Predicate::Neq(key, v)),
            (Self::Lt, AnyValue::Unit(v)) => Ok(Predicate::Lt(key, v)),
            (Self::Le, AnyValue::Unit(v)) => Ok(Predicate::Le(key, v)),
            (Self::Gt, AnyValue::Unit(v)) => Ok(Predicate::Gt(key, v)),
            (Self::Ge, AnyValue::Unit(v)) => Ok(Predicate::Ge(key, v)),
            (Self::In, AnyValue::Set(v)) => Ok(Predicate::In(key, v)),
            (Self::Like, AnyValue::Unit(v)) => Ok(Predicate::Like(key, v)),
            (Self::In, AnyValue::Unit(v)) => Err(Error::predicate_operand(self, "a set", v)),
            (_, AnyValue::Set(v)) => Err(Error::predicate_operand(self, "a single value", v)),
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
        }
    }
}

fn unescape_ident(lex: &Lexer<Token>) -> String {
    let escaped = lex.slice().chars().next().is_some_and(|x| x == '`');
    if escaped {
        let span = lex.span();
        lex.source()[span.start + 1..span.end - 1].replace("``", "`")
    } else {
        lex.slice().to_string()
    }
}

fn unescape_str(lex: &Lexer<Token>) -> String {
    let span = lex.span();
    lex.source()[span.start + 1..span.end - 1].replace("''", "'")
}

fn escape_str(s: &String) -> String {
    format!("'{}'", s.replace("'", "''"))
}

fn parse_int(lex: &Lexer<Token>) -> i64 {
    lex.slice().parse().unwrap()
}

fn parse_float(lex: &Lexer<Token>) -> f64 {
    lex.slice().parse().unwrap()
}

fn parse_bool(lex: &Lexer<Token>) -> bool {
    lex.slice().to_lowercase().parse().unwrap()
}

#[derive(Debug)]
struct Parser<'src> {
    lex: Lexer<'src, Token>,
    symbol_stack: Vec<Symbol>,
    state_stack: Vec<State>,
}

#[derive(Debug, Clone)]
enum Symbol {
    None,
    Ex(Predicate),
    E1(Predicate),
    E2(Predicate),
    V(AnyValue),
    S(SetValue),
    Si(HashSet<i64>),
    Ss(HashSet<String>),
    Token(Token),
}

#[derive(Debug, Clone, Copy)]
struct State(usize);

macro_rules! shift {
    ($self:ident, $token:ident, $state:literal) => {{
        let t = $token.unwrap();
        $token = $self.next_token()?;
        $self.symbol_stack.push(Symbol::Token(t));
        $self.state_stack.push(State($state));
    }};
}

macro_rules! reduce {
    ($self:ident, $r:literal) => {{
        let symbol = $self.reduce($r)?;
        $self.state_stack.truncate($self.symbol_stack.len());
        let state = $self.state_stack.last().unwrap();
        $self.state_stack.push(Self::goto(state, &symbol));
        $self.symbol_stack.push(symbol);
    }};
}

macro_rules! action_pattern {
    ($s:literal, end) => {
        (State($s), None)
    };
    ($s:literal, _) => {
        (State($s), _)
    };
    ($s:literal, $token:ident) => {
        (State($s), Some(Token::$token))
    };
    ($s:literal, $token:ident _) => {
        (State($s), Some(Token::$token(_)))
    };
}

macro_rules! action_arm {
    ($self:ident, $token:ident, shift, $ns:literal) => {
        shift!($self, $token, $ns)
    };
    ($self:ident, $token:ident, reduce, $ns:literal) => {
        reduce!($self, $ns)
    };
    ($self:ident, $token:ident, error, $err:literal) => {
        return Err(Error::syntax_error_expecting(
            $err,
            $self.lex.source(),
            $self.lex.span().start,
        ))
    };
    ($self:ident, $token:ident, accept, _) => {
        break
    };
}

macro_rules! action_table {
    ($([$s:literal, $($a:tt)*] = $t:tt $ns:tt)*) => {
        #[must_use]
        fn parse(mut self) -> Result<Predicate> {
            let mut token = self.next_token()?;

            while let Some(state) = self.state_stack.last() {
                match (state, token.as_ref()) {
                    $(action_pattern!($s, $($a)*) => action_arm!(self, token, $t, $ns),)*
                    _ => return Err(Error::syntax_error(self.lex.source(), self.lex.span().start)),
                }
            }

            if let Some(Symbol::Ex(expr)) = self.symbol_stack.pop() {
                Ok(expr)
            } else {
                Err(Error::syntax_error(self.lex.source(), self.lex.span().start))
            }
        }
    };
}

macro_rules! goto_table {
    ($([$s:literal, $a:ident] = $ns:literal)*) => {
        fn goto(state: &State, symbol: &Symbol) -> State {
            match (state, symbol) {
                $((State($s), Symbol::$a(_)) => State($ns),)*
                _ => unreachable!("invalid state transition {state:?} {symbol:?}"),
            }
        }
    };
}

macro_rules! attr {
    ($Variant:ident from $args:ident) => {
        match $args.pop().unwrap() {
            Symbol::$Variant(x) => x,
            _ => unreachable!("invalid symbol"),
        }
    };
}

macro_rules! token_attr {
    ($Variant:ident from $args:ident) => {
        match $args.pop().unwrap() {
            Symbol::Token(Token::$Variant(x)) => x,
            _ => unreachable!("invalid token"),
        }
    };
}

macro_rules! count_args {
    () => {
        0
    };
    ({ $v:expr }) => {
        0
    };
    (:$Rhs:ident(mut $s:ident) $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
    (:$Rhs:ident($s:ident) $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
    (:$Rhs:ident $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
    ($Rhs:ident(mut $s:ident) $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
    ($Rhs:ident($s:ident) $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
    ($Rhs:ident $($rest:tt) *) => {
        1 + count_args!($($rest)*)
    };
}

macro_rules! synthesize {
    ($args:ident; $Lhs:ident -> ) => {
        Symbol::$Lhs
    };
    ($args:ident; $Lhs:ident -> { $v:expr }) => {
        Symbol::$Lhs($v)
    };
    ($args:ident; $Lhs:ident -> :$Rhs:ident(mut $s:ident) $($rest:tt) *) => {{
        let mut $s = token_attr!($Rhs from $args);
        synthesize!($args; $Lhs -> $($rest)*)
    }};
    ($args:ident; $Lhs:ident -> :$Rhs:ident($s:ident) $($rest:tt) *) => {{
        let $s = token_attr!($Rhs from $args);
        synthesize!($args; $Lhs -> $($rest)*)
    }};
    ($args:ident; $Lhs:ident -> :$Rhs:ident $($rest:tt) *) => {{
        $args.pop();
        synthesize!($args; $Lhs -> $($rest)*)
    }};
    ($args:ident; $Lhs:ident -> $Rhs:ident(mut $s:ident) $($rest:tt) *) => {{
        let mut $s = attr!($Rhs from $args);
        synthesize!($args; $Lhs -> $($rest)*)
    }};
    ($args:ident; $Lhs:ident -> $Rhs:ident($s:ident) $($rest:tt) *) => {{
        let $s = attr!($Rhs from $args);
        synthesize!($args; $Lhs -> $($rest)*)
    }};
    ($args:ident; $Lhs:ident -> $Rhs:ident $($rest:tt) *) => {{
        $args.pop();
        synthesize!($args; $Lhs -> $($rest)*)
    }};
}

macro_rules! reduce_rules {
    ($($i:literal: $Lhs:ident -> [$($rest:tt)*] )*) => {
        fn reduce(&mut self, i: usize) -> Result<Symbol> {
            match i {
                $($i => {
                    let n = count_args!($($rest) *);
                    let mut _args = self.pop_symbols(n);
                    Ok(synthesize!(_args; $Lhs -> $($rest) *))
                },)*
                _ => unreachable!("invalid reduction"),
            }
        }
    };
}

impl<'src> Parser<'src> {
    fn new(source: &'src str) -> Self {
        Parser {
            lex: Lexer::new(source),
            symbol_stack: vec![Symbol::None],
            state_stack: vec![State(0)],
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        let output = self.lex.next();
        match output {
            Some(Ok(x)) => Ok(Some(x)),
            Some(Err(_)) => Err(Error::scan(self.lex.slice())),
            None => Ok(None),
        }
    }

    fn pop_symbols(&mut self, n: usize) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        for _ in 0..n {
            symbols.push(self.symbol_stack.pop().unwrap());
        }
        symbols
    }

    action_table! {
        [ 0, ParenO] = shift 4
        [ 0, Not] = shift 5
        [ 0, Key _] = shift 6
        [ 0, _] = error "an expression"
        [ 1, Or] = shift 7
        [ 1, end] = accept _
        [ 1, _] = error "OR or end of input"
        [ 2, ParenC] = reduce 2
        [ 2, And] = shift 8
        [ 2, Or] = reduce 2
        [ 2, end] = reduce 2
        [ 2, _] = error "AND, OR, `)` or end of expression"
        [ 3, ParenC] = reduce 4
        [ 3, And] = reduce 4
        [ 3, Or] = reduce 4
        [ 3, end] = reduce 4
        [ 3, _] = error "AND, OR, `)` or end of expression"
        [ 4, ParenO] = shift 4
        [ 4, Not] = shift 5
        [ 4, Key _] = shift 6
        [ 4, _] = error "an expression"
        [ 5, ParenO] = shift 4
        [ 5, Not] = shift 5
        [ 5, Key _] = shift 6
        [ 5, _] = error "an expression"
        [ 6, Op _] = shift 14
        [ 6, _] = error "an operator"
        [ 7, ParenO] = shift 4
        [ 7, Not] = shift 5
        [ 7, Key _] = shift 6
        [ 7, _] = error "an expression"
        [ 8, ParenO] = shift 4
        [ 8, Not] = shift 5
        [ 8, Key _] = shift 6
        [ 8, _] = error "an expression"
        [ 9, ParenC] = shift 13
        [ 9, Or] = shift 7
        [ 9, _] = error "OR or `)`"
        [10, ParenC] = reduce 6
        [10, And] = reduce 6
        [10, Or] = reduce 6
        [10, end] = reduce 6
        [10, _] = error "AND, OR, `)` or end of expression"
        [11, ParenC] = reduce 1
        [11, And] = shift 8
        [11, Or] = reduce 1
        [11, end] = reduce 1
        [11, _] = error "AND, OR, `)` or end of expression"
        [12, ParenC] = reduce 3
        [12, And] = reduce 3
        [12, Or] = reduce 3
        [12, end] = reduce 3
        [12, _] = error "AND, OR, `)` or end of expression"
        [13, ParenC] = reduce 5
        [13, And] = reduce 5
        [13, Or] = reduce 5
        [13, end] = reduce 5
        [13, _] = error "AND, OR, `)` or end of expression"
        [14, ParenO] = shift 21
        [14, ValInt _] = shift 16
        [14, ValStr _] = shift 20
        [14, ValFloat _] = shift 17
        [14, ValBool _] = shift 18
        [14, ValNil] = shift 19
        [14, _] = error "a value"
        [15, ParenC] = reduce 7
        [15, And] = reduce 7
        [15, Or] = reduce 7
        [15, end] = reduce 7
        [15, _] = error "AND, OR, `)` or end of expression"
        [16, ParenC] = reduce 15
        [16, And] = reduce 15
        [16, Or] = reduce 15
        [16, end] = reduce 15
        [16, _] = error "AND, OR, `)` or end of expression"
        [17, ParenC] = reduce 17
        [17, And] = reduce 17
        [17, Or] = reduce 17
        [17, end] = reduce 17
        [17, _] = error "AND, OR, `)` or end of expression"
        [18, ParenC] = reduce 18
        [18, And] = reduce 18
        [18, Or] = reduce 18
        [18, end] = reduce 18
        [18, _] = error "AND, OR, `)` or end of expression"
        [19, ParenC] = reduce 19
        [19, And] = reduce 19
        [19, Or] = reduce 19
        [19, end] = reduce 19
        [19, _] = error "AND, OR, `)` or end of expression"
        [20, ParenC] = reduce 16
        [20, And] = reduce 16
        [20, Or] = reduce 16
        [20, end] = reduce 16
        [20, _] = error "AND, OR, `)` or end of expression"
        [21, ValInt _] = reduce 12
        [21, ValStr _] = reduce 14
        [21, _] = error "an integer or a string"
        [22, ParenC] = shift 23
        [22, _] = error "`)`"
        [23, ParenC] = reduce 8
        [23, And] = reduce 8
        [23, Or] = reduce 8
        [23, end] = reduce 8
        [23, _] = error "AND, OR, `)` or end of expression"
        [24, ValInt _] = shift 25
        [24, _] = error "an integer"
        [25, Comma] = shift 26
        [25, ParenC] = reduce 9
        [25, _] = error "`,` or `)`"
        [26, ValInt _] = reduce 11
        [26, _] = error "an integer"
        [27, ValStr _] = shift 28
        [27, _] = error "a string"
        [28, Comma] = shift 29
        [28, ParenC] = reduce 9
        [28, _] = error "`,` or `)`"
        [29, ValStr _] = reduce 13
        [29, _] = error "a string"
    }

    goto_table! {
        [ 0, Ex] =  1
        [ 0, E1] =  2
        [ 0, E2] =  3
        [ 4, Ex] =  9
        [ 4, E1] =  2
        [ 4, E2] =  3
        [ 5, E2] = 10
        [ 7, E1] = 11
        [ 7, E2] =  3
        [ 8, E2] = 12
        [14,  V] = 15
        [21,  S] = 22
        [21, Si] = 24
        [21, Ss] = 27
    }

    reduce_rules! {
        1:  Ex -> [ Ex(p1) :Or E1(p2) { p1 | p2 } ]
        2:  Ex -> [ E1(p) { p } ]
        3:  E1 -> [ E1(p1) :And E2(p2) { p1 & p2 } ]
        4:  E1 -> [ E2(p) { p } ]
        5:  E2 -> [ :ParenO Ex(p) :ParenC { p } ]
        6:  E2 -> [ :Not E2(p) { !p } ]
        7:  E2 -> [ :Key(key) :Op(op) V(val) { op.predicate(key, val)? } ]
        8:  V  -> [ :ParenO S(s) :ParenC { AnyValue::Set(s) } ]
        9:  S  -> [ Si(mut s) :ValInt(v) {{ s.insert(v); SetValue::IntSet(s) }} ]
        10: S  -> [ Ss(mut s) :ValStr(v) {{ s.insert(v); SetValue::StrSet(s) }} ]
        11: Si -> [ Si(mut s) :ValInt(v) :Comma {{ s.insert(v); s }} ]
        12: Si -> [ { HashSet::new() } ]
        13: Ss -> [ Ss(mut s) :ValStr(v) :Comma {{ s.insert(v); s }} ]
        14: Ss -> [ { HashSet::new() } ]
        15: V  -> [ :ValInt(v) { AnyValue::Unit(Value::Int(v)) } ]
        16: V  -> [ :ValStr(v) { AnyValue::Unit(Value::Str(v)) } ]
        17: V  -> [ :ValFloat(v) { AnyValue::Unit(Value::Float(v)) } ]
        18: V  -> [ :ValBool(v) { AnyValue::Unit(Value::Bool(v)) } ]
        19: V  -> [ :ValNil { AnyValue::Unit(Value::Nil) } ]
    }
}
