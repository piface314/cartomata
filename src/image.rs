//! Image backend implementations.

mod blend;
mod color;
mod stroke;

use std::path::Path;

use cairo::{Format, ImageSurface};
use libvips::{ops, VipsApp, VipsImage};
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
pub use crate::image::blend::BlendMode;
pub use crate::image::color::Color;
pub use crate::image::stroke::Stroke;
use crate::text::FontManager;

pub struct ImgBackend<'f> {
    vips_app: VipsApp,
    font_manager: FontManager<'f>,
}

#[derive(Debug, Copy, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
pub enum FitMode {
    Contain,
    Cover,
    Stretch,
}

impl Default for FitMode {
    fn default() -> Self {
        Self::Cover
    }
}

#[derive(Debug, Copy, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
pub enum Origin {
    Absolute,
    Relative,
}

impl Default for Origin {
    fn default() -> Self {
        Self::Relative
    }
}

impl<'f> ImgBackend<'f> {
    pub fn new(vips_app: VipsApp, font_manager: FontManager<'f>) -> Self {
        Self {
            vips_app,
            font_manager,
        }
    }

    fn err(&self, e: libvips::error::Error) -> Error {
        Error::VipsError(format!(
            "{e}\n{}",
            self.vips_app.error_buffer().expect("vips error buffer")
        ))
    }

    fn reinterpret(&self, img: &VipsImage) -> Result<VipsImage> {
        let img = ops::copy_with_opts(
            &img,
            &ops::CopyOptions {
                interpretation: ops::Interpretation::Srgb,
                width: img.get_width(),
                height: img.get_height(),
                bands: img.get_bands(),
                format: ops::BandFormat::Uchar,
                ..Default::default()
            },
        )
        .map_err(|e| self.err(e))?;
        if img.get_bands() == 3 {
            ops::bandjoin_const(&img, &mut [255.0]).map_err(|e| self.err(e))
        } else {
            Ok(img)
        }
    }

    pub fn new_canvas(&self, bg: &Color, width: usize, height: usize) -> Result<VipsImage> {
        let (r, g, b, a) = bg.scaled_rgba();
        let img =
            ops::black_with_opts(width as i32, height as i32, &ops::BlackOptions { bands: 4 })
                .map_err(|e| self.err(e))?;
        let img = VipsImage::new_from_image(&img, &[r, g, b, a]).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    fn vips_to_cairo(&self, img: &VipsImage) -> Result<ImageSurface> {
        let data = img.image_write_to_memory();
        let stride = (data.len() / img.get_height() as usize) as i32;
        ImageSurface::create_for_data(
            data,
            Format::ARgb32,
            img.get_width(),
            img.get_height(),
            stride,
        )
        .map_err(|_| Error::ImageConversionError("vips", "cairo"))
    }

    fn cairo_to_vips(&self, img: &mut ImageSurface) -> Result<VipsImage> {
        let (w, h) = (img.width(), img.height());
        let data = img
            .data()
            .map_err(|_| Error::ImageConversionError("cairo", "vips"))?;
        let img = VipsImage::new_from_memory(&data, w, h, 4, ops::BandFormat::Uchar)
            .map_err(|_| Error::ImageConversionError("cairo", "vips"))?;
        ops::copy_with_opts(
            &img,
            &ops::CopyOptions {
                interpretation: ops::Interpretation::Srgb,
                width: img.get_width(),
                height: img.get_height(),
                bands: img.get_bands(),
                ..Default::default()
            },
        )
        .map_err(|_| Error::ImageConversionError("cairo", "vips"))
    }

    pub fn load_image(&self, fp: impl AsRef<Path>) -> Result<VipsImage> {
        let fp = fp.as_ref();
        let img = VipsImage::new_from_file(&fp.to_string_lossy()).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn scale(&self, img: &VipsImage, sx: f64, sy: f64) -> Result<VipsImage> {
        ops::resize_with_opts(
            &img,
            sx,
            &ops::ResizeOptions {
                vscale: sy,
                ..Default::default()
            },
        )
        .map_err(|e| self.err(e))
    }

    pub fn scale_to(&self, img: &VipsImage, w: Option<f64>, h: Option<f64>) -> Result<VipsImage> {
        let (iw, ih) = (img.get_width() as f64, img.get_height() as f64);
        let (sx, sy) = match (w, h) {
            (Some(rw), Some(rh)) => (rw / iw, rh / ih),
            (Some(rw), None) => {
                let s = rw / iw;
                (s, s)
            }
            (None, Some(rh)) => {
                let s = rh / ih;
                (s, s)
            }
            (None, None) => (1.0, 1.0),
        };
        self.scale(img, sx, sy)
    }

    pub fn scale_to_fit(
        &self,
        img: &VipsImage,
        w: f64,
        h: f64,
        mode: FitMode,
    ) -> Result<VipsImage> {
        let (iw, ih) = (img.get_width() as f64, img.get_height() as f64);
        let aspect_ratio = iw / ih;
        let (sx, sy) = match mode {
            FitMode::Contain | FitMode::Cover => {
                if (aspect_ratio < 1.0) ^ (mode == FitMode::Contain) {
                    let s = w as f64 / iw;
                    (s, s)
                } else {
                    let s = h as f64 / ih;
                    (s, s)
                }
            }
            FitMode::Stretch => (1.0, 1.0),
        };
        self.scale(img, sx, sy)
    }

    pub fn rotate(
        &self,
        img: &VipsImage,
        deg: f64,
        ox: f64,
        oy: f64,
        origin: Origin,
    ) -> Result<(VipsImage, f64, f64)> {
        let (w, h) = (img.get_width() as f64, img.get_height() as f64);
        let (ox, oy) = match origin {
            Origin::Absolute => (ox, oy),
            Origin::Relative => (ox * w, oy * h),
        };
        let img = ops::rotate(&img, deg).map_err(|e| self.err(e))?;
        let (tw, th) = (img.get_width() as f64, img.get_height() as f64);
        let (sin, cos) = deg.to_radians().sin_cos();
        let (dx, dy) = (
            (ox - 0.5 * w) * cos - (oy - 0.5 * h) * sin + 0.5 * tw,
            (ox - 0.5 * w) * sin + (oy - 0.5 * h) * cos + 0.5 * th,
        );
        Ok((img, dx, dy))
    }

    pub fn stroke(&self, img: &VipsImage, stroke: Stroke) -> Result<VipsImage> {
        let Stroke(radius, color) = stroke;
        let mask = ops::black(radius * 2 + 1, radius * 2 + 1).map_err(|e| self.err(e))?;
        let mask = ops::add(
            &mask,
            &VipsImage::new_from_image1(&mask, 128.0).map_err(|e| self.err(e))?,
        )
        .map_err(|e| self.err(e))?;
        ops::draw_circle_with_opts(
            &mask,
            &mut [255.0],
            radius,
            radius,
            radius,
            &ops::DrawCircleOptions { fill: true },
        )
        .map_err(|e| self.err(e))?;

        let (w, h) = (img.get_width(), img.get_height());
        let img = ops::embed(&img, radius, radius, w + 2 * radius, h + 2 * radius)
            .map_err(|e| self.err(e))?;

        let alpha = ops::extract_band(&img, 3).map_err(|e| self.err(e))?;
        let alpha =
            ops::morph(&alpha, &mask, ops::OperationMorphology::Dilate).map_err(|e| self.err(e))?;
        let alpha = ops::gaussblur_with_opts(
            &alpha,
            0.5,
            &ops::GaussblurOptions {
                min_ampl: 0.2,
                ..Default::default()
            },
        )
        .map_err(|e| self.err(e))?;

        let (r, g, b) = color.scaled_rgb();
        let stroke = VipsImage::new_from_image(&alpha, &[r, g, b]).map_err(|e| self.err(e))?;
        let stroke = ops::bandjoin(&mut [stroke, alpha]).map_err(|e| self.err(e))?;
        let stroke = self.reinterpret(&stroke)?;
        let img = ops::composite_2(&stroke, &img, ops::BlendMode::Over).map_err(|e| self.err(e))?;
        Ok(img)
    }

    pub fn overlay(
        &self,
        base: &VipsImage,
        src: &VipsImage,
        x: i32,
        y: i32,
        ox: f64,
        oy: f64,
        origin: Origin,
        mode: BlendMode,
    ) -> Result<VipsImage> {
        let (bw, bh) = (base.get_width(), base.get_height());
        let (w, h) = (src.get_width() as f64, src.get_height() as f64);
        let (ox, oy) = match origin {
            Origin::Absolute => (ox as i32, oy as i32),
            Origin::Relative => ((ox * w) as i32, (oy * h) as i32),
        };
        let src = ops::embed(&src, x - ox, y - oy, bw, bh).map_err(|e| self.err(e))?;
        ops::composite_2(&base, &src, mode.into()).map_err(|e| self.err(e))
    }
}
