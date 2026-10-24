use crate::{image::Unit, layer::LayerOut};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(untagged)]
pub enum Position {
    Absolute(f64),
    Relative(RelativePosition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Default)]
pub enum Axis {
    #[default]
    X,
    Y,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RelativePosition {
    pub layer: isize,
    pub axis: Axis,
    #[serde(default)]
    pub origin: Unit,
    #[serde(default)]
    pub offset: f64,
}

macro_rules! from_castable {
    ($T:ty) => {
        impl From<$T> for Position {
            fn from(value: $T) -> Self {
                Position::Absolute(value as f64)
            }
        }
    };
}

from_castable!(i32);
from_castable!(i64);
from_castable!(u32);
from_castable!(u64);
from_castable!(f32);
from_castable!(f64);

impl From<RelativePosition> for Position {
    fn from(value: RelativePosition) -> Self {
        Self::Relative(value)
    }
}
impl Position {
    pub fn apply(&self, layers: impl AsRef<[LayerOut]>) -> f64 {
        match self {
            Self::Absolute(x) => *x,
            Self::Relative(r) => r.apply(layers).unwrap_or_default()
        }
    }
}

impl RelativePosition {
    pub fn last_x(offset: f64) -> Self {
        Self::relative_x(-1, offset)
    }

    pub fn last_y(offset: f64) -> Self {
        Self::relative_y(-1, offset)
    }

    pub fn relative_x(layer: isize, offset: f64) -> Self {
        Self { layer, axis: Axis::X, origin: Default::default(), offset }
    }

    pub fn relative_y(layer: isize, offset: f64) -> Self {
        Self { layer, axis: Axis::Y, origin: Default::default(), offset }
    }

    pub fn with_origin(mut self, origin: Unit) -> Self {
        self.origin = origin;
        self
    }

    pub fn apply(&self, layers: impl AsRef<[LayerOut]>) -> Option<f64> {
        let RelativePosition { layer: i, origin, offset, axis } = self;
        let layers = layers.as_ref();
        let n = layers.len() as isize;
        let index = if *i >= 0 { *i } else { n + i };
        let layer = layers.get(index as usize)?;
        let (p, d) = match axis {
            Axis::X => (layer.x, layer.img.get_width()),
            Axis::Y => (layer.y, layer.img.get_height()),
        };
        Some((p as f64) + origin.apply(d as f64) + offset)
    }
}
