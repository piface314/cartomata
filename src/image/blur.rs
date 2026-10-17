use crate::error::ImgError;
use crate::layer::{LayerFx, LayerOut, RenderContext};

#[derive(Debug, Clone, Copy)]
pub struct Blur(f64);

impl From<f64> for Blur {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Blur> for LayerFx {
    fn from(blur: Blur) -> Self {
        let desc = format!("{blur:?}");
        LayerFx::new(desc, move |ctx: &RenderContext, out: LayerOut| -> Result<LayerOut, ImgError> {
            let LayerOut { img, x, y, blend } = out;
            let (img, dx, dy) = ctx.backend.blur_checked(img, blur.0)?;
            let x = (x as f64 - dx).round() as i32;
            let y = (y as f64 - dy).round() as i32;
            Ok(LayerOut { img, x, y, blend })
        })
    }
}
