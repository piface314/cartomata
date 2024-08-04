//! Stroke parameter definitions

use crate::image::color::Color;

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Stroke {
    pub size: i32,
    pub color: Color,
}
