//! Image backend implementations.

mod color;
mod stroke;

use std::fs::File;
use std::path::Path;

use cairo::{Context, Format, ImageSurface};
use libvips::{ops, VipsImage};
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
pub use crate::image::color::Color;
pub use crate::image::stroke::Stroke;
use crate::text::FontManager;

pub struct ImgBackend<'f> {
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

impl<'f> ImgBackend<'f> {
    pub fn new(font_manager: FontManager<'f>) -> Self {
        Self { font_manager }
    }

    pub fn new_canvas(
        &self,
        bg: &Color,
        width: usize,
        height: usize,
    ) -> Result<(ImageSurface, Context)> {
        let (r, g, b, a) = bg.rgba();
        let img = ImageSurface::create(Format::ARgb32, width as i32, height as i32)
            .map_err(|e| Error::CairoError(e.to_string()))?;
        let cr = Context::new(&img).map_err(|e| Error::CairoError(e.to_string()))?;
        cr.set_source_rgba(r, g, b, a);
        cr.paint().map_err(|e| Error::CairoError(e.to_string()))?;
        Ok((img, cr))
    }

    fn vips_to_cairo(img: &VipsImage) -> Result<ImageSurface> {
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

    fn cairo_to_vips(img: &mut ImageSurface) -> Result<VipsImage> {
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

    pub fn load_image(&self, fp: impl AsRef<Path>) -> Result<ImageSurface> {
        let path = fp.as_ref();
        let mut reader = File::open(&path)
            .map_err(|e| Error::FailedOpenImage(path.display().to_string(), e.to_string()))?;
        ImageSurface::create_from_png(&mut reader)
            .map_err(|e| Error::FailedOpenImage(path.display().to_string(), e.to_string()))
    }

    pub fn target_size(
        &self,
        img: &ImageSurface,
        w: Option<f64>,
        h: Option<f64>,
    ) -> (f64, f64, f64, f64) {
        let (iw, ih) = (img.width() as f64, img.height() as f64);
        match (w, h) {
            (Some(rw), Some(rh)) => (rw, rh, rw / iw, rh / ih),
            (Some(rw), None) => {
                let s = rw / iw;
                (rw, s * ih, s, s)
            }
            (None, Some(rh)) => {
                let s = rh / ih;
                (s * iw, rh, s, s)
            }
            (None, None) => (iw, ih, 1.0, 1.0),
        }
    }

    pub fn target_size_to_fit(
        &self,
        img: &ImageSurface,
        w: f64,
        h: f64,
        mode: FitMode,
    ) -> (f64, f64, f64, f64) {
        let (iw, ih) = (img.width() as f64, img.height() as f64);
        let aspect_ratio = iw / ih;
        match mode {
            FitMode::Contain | FitMode::Cover => {
                if (aspect_ratio < 1.0) ^ (mode == FitMode::Contain) {
                    let s = w as f64 / iw;
                    (w as f64, s * ih, s, s)
                } else {
                    let s = h as f64 / ih;
                    (s * iw, h as f64, s, s)
                }
            }
            FitMode::Stretch => (w as f64, h as f64, 1.0, 1.0),
        }
    }

    pub fn paint(
        &self,
        cr: &Context,
        img: &mut ImageSurface,
        x: f64,
        y: f64,
        sx: f64,
        sy: f64,
        ox: f64,
        oy: f64,
        r: f64,
        stroke: Option<Stroke>,
    ) -> Result<()> {
        cr.save().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.translate(x, y);
        let (sinr, cosr) = r.sin_cos();
        let (ox, oy) = (ox - (ox * cosr - oy * sinr), oy - (ox * sinr + oy * cosr));
        if let Some(Stroke(size, color)) = stroke {
            cr.translate(ox - size as f64, oy - size as f64);
            cr.rotate(r);
            let img = self.stroked(img, sx, sy, size, color)?;
            cr.set_source_surface(img, 0.0, 0.0)
                .map_err(|e| Error::CairoError(e.to_string()))?;
        } else {
            cr.translate(ox, oy);
            cr.rotate(r);
            cr.scale(sx, sy);
            cr.set_source_surface(img, 0.0, 0.0)
                .map_err(|e| Error::CairoError(e.to_string()))?;
        }
        cr.paint().map_err(|e| Error::CairoError(e.to_string()))?;
        cr.restore().map_err(|e| Error::CairoError(e.to_string()))?;
        Ok(())
    }

    fn stroked(
        &self,
        img: &mut ImageSurface,
        sx: f64,
        sy: f64,
        radius: i32,
        color: Color,
    ) -> Result<ImageSurface> {
        let img = Self::cairo_to_vips(img)?;
        let err = |e: libvips::error::Error| Error::VipsError(e.to_string());

        let mask = ops::black(radius * 2 + 1, radius * 2 + 1).map_err(err)?;
        let mask = ops::add(
            &mask,
            &VipsImage::new_from_image1(&mask, 128.0).map_err(err)?,
        )
        .map_err(err)?;
        ops::draw_circle_with_opts(
            &mask,
            &mut [255.0],
            radius,
            radius,
            radius,
            &ops::DrawCircleOptions { fill: true },
        )
        .map_err(err)?;

        let img = ops::resize_with_opts(
            &img,
            sx,
            &ops::ResizeOptions {
                vscale: sy,
                ..Default::default()
            },
        )
        .map_err(err)?;
        let (w, h) = (img.get_width(), img.get_height());
        let img = ops::embed(&img, radius, radius, w + 2 * radius, h + 2 * radius).map_err(err)?;

        let alpha = ops::extract_band(&img, 3).map_err(err)?;
        let alpha = ops::morph(&alpha, &mask, ops::OperationMorphology::Dilate).map_err(err)?;
        let alpha = ops::gaussblur_with_opts(
            &alpha,
            0.5,
            &ops::GaussblurOptions {
                min_ampl: 0.2,
                ..Default::default()
            },
        )
        .map_err(err)?;

        let (r, g, b) = color.rgb();
        let r = VipsImage::new_from_image1(&alpha, r * 255.0).map_err(err)?;
        let g = VipsImage::new_from_image1(&alpha, g * 255.0).map_err(err)?;
        let b = VipsImage::new_from_image1(&alpha, b * 255.0).map_err(err)?;
        let stroke = ops::bandjoin(&mut [r, g, b, alpha]).map_err(err)?;
        let stroke = ops::copy_with_opts(
            &stroke,
            &ops::CopyOptions {
                interpretation: ops::Interpretation::Srgb,
                width: stroke.get_width(),
                height: stroke.get_height(),
                bands: stroke.get_bands(),
                ..Default::default()
            },
        )
        .map_err(err)?;
        let img = ops::composite_2(&stroke, &img, ops::BlendMode::Over).map_err(err)?;
        Self::vips_to_cairo(&img)
    }
}
