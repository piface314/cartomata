pub use serde_json::{Map, Number, Value};
use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq, Default)]
pub enum ValueRef<'v> {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(&'v str),
    ArrayRef(&'v Vec<Value>),
    Array(Vec<ValueRef<'v>>),
    ObjectRef(&'v Map<String, Value>),
    Object(HashMap<&'v str, ValueRef<'v>>),
}

impl<'v> From<&'v Value> for ValueRef<'v> {
    fn from(value: &'v Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(v) => Self::Bool(*v),
            Value::Number(v) => Self::Number(v.clone()),
            Value::String(v) => Self::String(v.as_str()),
            Value::Array(v) => Self::ArrayRef(v),
            Value::Object(v) => Self::ObjectRef(v),
        }
    }
}

macro_rules! value_int_from {
    ($($V:ty)+) => {
        $(
            impl<'v> From<$V> for ValueRef<'v> {
                fn from(value: $V) -> Self {
                    Self::from(Number::from(value))
                }
            }

            impl<'v> From<&$V> for ValueRef<'v> {
                fn from(value: &$V) -> Self {
                    Self::from(Number::from(*value))
                }
            }
        )*
    };
}

value_int_from!(i64 i32 i16 i8 u64 u32 u16 u8);

macro_rules! value_float_from {
    ($($V:ty)+) => {
        $(
            impl<'v> From<$V> for ValueRef<'v> {
                fn from(value: $V) -> Self {
                    Number::from_f64(value as f64).map(|x| Self::from(x)).unwrap_or_default()
                }
            }

            impl<'v> From<&$V> for ValueRef<'v> {
                fn from(value: &$V) -> Self {
                    Number::from_f64(*value as f64).map(|x| Self::from(x)).unwrap_or_default()
                }
            }
        )*
    };
}

value_float_from!(f64 f32);

impl<'v> From<bool> for ValueRef<'v> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}


impl<'v> From<&bool> for ValueRef<'v> {
    fn from(value: &bool) -> Self {
        Self::Bool(*value)
    }
}


impl<'v> From<Number> for ValueRef<'v> {
    fn from(value: Number) -> Self {
        Self::Number(value.clone())
    }
}

impl<'v> From<&Number> for ValueRef<'v> {
    fn from(value: &Number) -> Self {
        Self::Number(value.clone())
    }
}

impl<'v> From<&'v str> for ValueRef<'v> {
    fn from(value: &'v str) -> Self {
        ValueRef::String(value)
    }
}

impl<'v> From<&'v String> for ValueRef<'v> {
    fn from(value: &'v String) -> Self {
        ValueRef::String(value.as_str())
    }
}

impl<'v, T: Clone + Into<ValueRef<'v>>> From<&'v Vec<T>> for ValueRef<'v> {
    fn from(value: &'v Vec<T>) -> Self {
        ValueRef::Array(value.iter().map(|v| v.clone().into()).collect())
    }
}

impl<'v, T: Clone + Into<ValueRef<'v>>> From<&'v HashMap<String, T>> for ValueRef<'v> {
    fn from(value: &'v HashMap<String, T>) -> Self {
        ValueRef::Object(value.iter().map(|(k, v)| (k.as_str(), v.clone().into())).collect())
    }
}

impl<'v, T: Clone + Into<ValueRef<'v>>> From<&'v Option<T>> for ValueRef<'v> {
    fn from(value: &'v Option<T>) -> Self {
        value.as_ref().map(|v| v.clone().into()).unwrap_or_default()
    }
}

impl<'v> PartialOrd for ValueRef<'v> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => {
                if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
                    a.partial_cmp(&b)
                } else if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
                    a.partial_cmp(&b)
                } else if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                    a.partial_cmp(&b)
                } else {
                    None
                }
            }
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
            (Self::Array(a), Self::Array(b)) => a.partial_cmp(b),
            (_, _) => None,
        }
    }
}
