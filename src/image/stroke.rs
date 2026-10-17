//! Stroke parameter definitions

use crate::error::ImgError;
use crate::image::color::Color;
use crate::layer::{LayerFx, LayerOut, RenderContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Stroke {
    pub size: i32,
    pub color: Color,
}

impl From<Stroke> for LayerFx {
    fn from(stroke: Stroke) -> Self {
        let desc = format!("{stroke:?}");
        Self::new(
            desc,
            move |ctx: &RenderContext, out: LayerOut| -> Result<LayerOut, ImgError> {
                let LayerOut { img, x, y, blend } = out;
                let d = stroke.size;
                let img = ctx.backend.stroke_checked(img, stroke)?;
                Ok(LayerOut { img, x: x - d, y: y - d, blend })
            },
        )
    }
}
