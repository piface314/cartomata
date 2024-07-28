//! Implements text markup parsing and related utility functions.

pub mod attr;
pub mod font;

pub use crate::text::font::FontManager;
use crate::error::{Error, Result};
use crate::text::attr::{IconAttr, ImgAttr, SpanAttr};

use logos::{Lexer, Logos};
use regex::Regex;
use std::fmt;

#[derive(Logos, Debug, PartialEq, Eq, Copy, Clone)]
enum TextToken {
    #[token("<")]
    TagOpen,
    #[token(">")]
    TagClose,
    #[regex(r#"([^<>]|\\[<>])+"#)]
    Text,
}

#[derive(Logos, Debug, PartialEq, Eq, Copy, Clone)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[token("<")]
    TagOpen,
    #[token("span")]
    TypeSpan,
    #[token("img")]
    TypeImg,
    #[token("icon")]
    TypeIcon,
    #[regex("[a-z][a-z0-9-]*")]
    Key,
    #[token("=")]
    Eq,
    #[regex(r#""([^"]|\\["])*""#)]
    Value,
    #[token("/")]
    TagSep,
    #[token(">")]
    TagClose,
    Text,
}

impl Into<Token> for TextToken {
    fn into(self) -> Token {
        match self {
            Self::TagOpen => Token::TagOpen,
            Self::TagClose => Token::TagClose,
            Self::Text => Token::Text,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TagOpen => write!(f, "<"),
            Self::TypeSpan => write!(f, "`span`"),
            Self::TypeImg => write!(f, "`img`"),
            Self::TypeIcon => write!(f, "`icon`"),
            Self::Key => write!(f, "a key"),
            Self::Eq => write!(f, "="),
            Self::Value => write!(f, "a value"),
            Self::TagSep => write!(f, "/"),
            Self::TagClose => write!(f, ">"),
            Self::Text => write!(f, "text"),
        }
    }
}

pub fn escape(text: &str) -> String {
    let re = Regex::new(r"([<>])").unwrap();
    re.replace_all(text, r"\$1").to_string()
}

pub fn unescape(text: &str) -> String {
    let re = Regex::new(r"\\([<>])").unwrap();
    re.replace_all(text, r"$1").to_string()
}

fn unescape_val(text: &str) -> String {
    let len = text.len();
    let re = Regex::new(r#"\\""#).unwrap();
    re.replace_all(&text[1..len - 1], "\"").to_string()
}

/// Parses text into text fragments that can be later turned
/// into a text layout.
// Grammar:
// (Markup) M → ϵ | text M | < T M
// (Tag)    T → span A / M > | img A / > | icon A / >
// (Attrs)  A → ϵ | key = value A
pub struct Parser<'src> {
    text_lexer: Lexer<'src, TextToken>,
    tag_lexer: Lexer<'src, Token>,
    lexer_context: LexerContext,
}

#[derive(Debug, Copy, Clone)]
enum LexerContext {
    Free,
    Tag,
}

#[derive(Debug, Copy, Clone)]
enum Symbol {
    M,
    T,
    A,
    Token(Token),
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::M => write!(f, "text, <, >, or end of input"),
            Self::T => write!(f, "a tag name"),
            Self::A => write!(f, "an attribute or /"),
            Self::Token(token) => token.fmt(f),
        }
    }
}

impl<'src> Parser<'src> {
    pub fn new(markup: &'src str) -> Self {
        let text_lexer = TextToken::lexer(markup);
        let tag_lexer = Token::lexer(markup);
        Self {
            text_lexer,
            tag_lexer,
            lexer_context: LexerContext::Free,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        let output = match self.lexer_context {
            LexerContext::Free => self.text_lexer.next().map(|r| {
                r.map_err(|_| Error::TextScanError(format!("{:?}", self.text_lexer.slice())))
                    .map(|t| t.into())
            }),
            LexerContext::Tag => self.tag_lexer.next().map(|r| {
                r.map_err(|_| Error::TextScanError(format!("{:?}", self.tag_lexer.slice())))
            }),
        };
        match output {
            Some(Ok(x)) => Ok(Some(x)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    fn set_context(&mut self, context: LexerContext) {
        match (self.lexer_context, context) {
            (LexerContext::Free, LexerContext::Tag) => self
                .tag_lexer
                .bump(self.text_lexer.span().end - self.tag_lexer.span().end),
            (LexerContext::Tag, LexerContext::Free) => self
                .text_lexer
                .bump(self.tag_lexer.span().end - self.text_lexer.span().end),
            _ => return,
        };
        self.lexer_context = context;
    }

    fn slice(&self) -> &'src str {
        match self.lexer_context {
            LexerContext::Free => self.text_lexer.slice(),
            LexerContext::Tag => self.tag_lexer.slice(),
        }
    }

    #[must_use]
    pub fn parse(mut self) -> Result<Markup> {
        let mut elems: Vec<Markup> = vec![Markup::Root(Vec::new())];
        let mut token = self.next_token()?;
        let mut stack = vec![Symbol::M];
        let mut last_key: Option<&'src str> = None;
        while let Some(top) = stack.pop() {
            match (top, token) {
                (Symbol::Token(x), Some(a)) => {
                    if x == a {
                        match a {
                            Token::TagSep => self.set_context(LexerContext::Free),
                            Token::Key => last_key = Some(self.slice()),
                            Token::Value => {
                                let key = last_key.expect("a key previously matched");
                                let slice = self.slice();
                                let val = &unescape_val(slice);
                                let tag = elems.last_mut().unwrap();
                                tag.push_attr(key, val)?;
                            }
                            Token::TagClose => {
                                let tag = elems.pop().unwrap();
                                elems.last_mut().unwrap().push_elem(tag);
                            }
                            _ => {}
                        }
                        token = self.next_token()?;
                    } else {
                        return Err(Error::TextUnexpected(x.to_string(), a.to_string()));
                    }
                }
                (Symbol::M, Some(Token::Text)) => {
                    // M → text M
                    let text = unescape(self.slice());
                    elems.last_mut().unwrap().push_elem(Markup::Text(text));
                    stack.extend([Symbol::M, Symbol::Token(Token::Text)]);
                }
                (Symbol::M, Some(Token::TagOpen)) => {
                    // M → < T M
                    self.set_context(LexerContext::Tag);
                    stack.extend([Symbol::M, Symbol::T, Symbol::Token(Token::TagOpen)]);
                }
                (Symbol::M, Some(Token::TagClose) | None) => {
                    // M → ϵ
                }
                (Symbol::T, Some(Token::TypeSpan)) => {
                    // T → span A / M >
                    elems.push(Markup::SpanTag(Vec::new(), Vec::new()));
                    stack.extend([
                        Symbol::M,
                        Symbol::Token(Token::TagClose),
                        Symbol::M,
                        Symbol::Token(Token::TagSep),
                        Symbol::A,
                        Symbol::Token(Token::TypeSpan),
                    ]);
                }
                (Symbol::T, Some(Token::TypeImg)) => {
                    // T → img A / >
                    elems.push(Markup::ImgTag(Vec::new()));
                    stack.extend([
                        Symbol::Token(Token::TagClose),
                        Symbol::Token(Token::TagSep),
                        Symbol::A,
                        Symbol::Token(Token::TypeImg),
                    ]);
                }
                (Symbol::T, Some(Token::TypeIcon)) => {
                    // T → icon A / >
                    elems.push(Markup::IconTag(Vec::new()));
                    stack.extend([
                        Symbol::Token(Token::TagClose),
                        Symbol::Token(Token::TagSep),
                        Symbol::A,
                        Symbol::Token(Token::TypeIcon),
                    ]);
                }
                (Symbol::A, Some(Token::Key)) => {
                    // A → key = value A
                    stack.extend([
                        Symbol::A,
                        Symbol::Token(Token::Value),
                        Symbol::Token(Token::Eq),
                        Symbol::Token(Token::Key),
                    ]);
                }
                (Symbol::A, Some(Token::TagSep)) => {
                    // A → ϵ
                }
                (symbol, Some(token)) => {
                    return Err(Error::TextUnexpected(symbol.to_string(), token.to_string()))
                }
                (symbol, None) => {
                    return Err(Error::TextUnexpected(
                        symbol.to_string(),
                        "end of input".to_string(),
                    ))
                }
            }
        }
        match (stack.last(), token) {
            (Some(symbol), Some(tk)) => Err(Error::TextUnexpected(
                symbol.to_string(),
                tk.to_string(),
            )),
            (Some(symbol), None) => Err(Error::TextUnexpected(
                symbol.to_string(),
                "end of input".to_string(),
            )),
            (None, Some(tk)) => Err(Error::TextUnexpected(
                "end of input".to_string(),
                tk.to_string(),
            )),
            (None, None) => Ok(elems.pop().unwrap()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Markup {
    Root(Vec<Markup>),
    Text(String),
    SpanTag(Vec<SpanAttr>, Vec<Markup>),
    ImgTag(Vec<ImgAttr>),
    IconTag(Vec<IconAttr>),
}

impl Markup {
    pub fn push_attr(&mut self, key: &str, value: &str) -> Result<()> {
        match self {
            Self::SpanTag(attrs, _) => attrs.push(SpanAttr::from_key_value(key, value)?),
            Self::ImgTag(attrs) => attrs.push(ImgAttr::from_key_value(key, value)?),
            Self::IconTag(attrs) => attrs.push(IconAttr::from_key_value(key, value)?),
            _ => unreachable!("trying to add attr to non tag"),
        };
        Ok(())
    }

    pub fn push_elem(&mut self, elem: Markup) {
        match self {
            Self::Root(v) => v.push(elem),
            Self::SpanTag(_, v) => v.push(elem),
            _ => unreachable!("trying to add elem to non span"),
        }
    }
}