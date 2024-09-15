//! Image backend implementations.

mod blend;
mod color;
mod map;
mod origin;
mod stroke;

use crate::error::{Error, Result};
pub use crate::image::blend::BlendMode;
pub use crate::image::color::Color;
pub use crate::image::map::ImageMap;
pub use crate::image::origin::{Origin, TextOrigin};
pub use crate::image::stroke::Stroke;
use crate::text::attr::{Gravity, ITagAttr, LayoutAttr};
use crate::text::{FontMap, Markup};

use cairo::ImageSurface;
use libvips::{ops, VipsApp, VipsImage};
use pango::prelude::FontMapExt;
#[cfg(feature = "cli")]
use serde::Deserialize;
use std::path::Path;

pub struct ImgBackend {
    vips_app: VipsApp,
}

#[derive(Debug, Copy, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "cli", derive(Deserialize))]
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

impl ImgBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            vips_app: libvips::VipsApp::default("cartomata").map_err(|e| Error::vips(e, None))?,
        })
    }

    fn err(&self, e: libvips::error::Error) -> Error {
        let extra = self.vips_app.error_buffer().ok();
        Error::vips(e, extra)
    }

    fn reinterpret(&self, img: &VipsImage) -> Result<VipsImage> {
        let img = ops::cast(&img, ops::BandFormat::Uchar).map_err(|e| self.err(e))?;
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

    pub fn create(&self, bg: &Color, width: i32, height: i32) -> Result<VipsImage> {
        let (r, g, b, a) = bg.scaled_rgba();
        let img = ops::black_with_opts(width, height, &ops::BlackOptions { bands: 4 })
            .map_err(|e| self.err(e))?;
        let img = VipsImage::new_from_image(&img, &[r, g, b, a]).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn cairo_to_vips(&self, img: ImageSurface) -> Result<VipsImage> {
        let mut buffer = Vec::new();
        img.write_to_png(&mut buffer)
            .map_err(Error::cairo_to_vips)?;
        let mut img = VipsImage::new_from_buffer(&buffer, "").map_err(|e| self.err(e))?;
        img.image_wio_input().map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn open(&self, fp: impl AsRef<str>) -> Result<VipsImage> {
        let fp = fp.as_ref();
        let img = VipsImage::new_from_file(fp).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn set_color(&self, img: &VipsImage, color: Color) -> Result<VipsImage> {
        let (r, g, b) = color.scaled_rgb();
        let rgb = VipsImage::new_from_image(img, &[r, g, b]).map_err(|e| self.err(e))?;
        let current_a = ops::extract_band(img, 3).map_err(|e| self.err(e))?;
        let a = match color.a {
            Some(alpha) => {
                let a = VipsImage::new_from_image1(img, alpha).map_err(|e| self.err(e))?;
                ops::multiply(&current_a, &a).map_err(|e| self.err(e))?
            }
            None => current_a,
        };
        let img = ops::bandjoin(&mut [rgb, a]).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn set_opacity(&self, img: &VipsImage, alpha: f64) -> Result<VipsImage> {
        let current = ops::extract_band(img, 3).map_err(|e| self.err(e))?;
        let a = VipsImage::new_from_image1(&img, alpha).map_err(|e| self.err(e))?;
        let a = ops::multiply(&current, &a).map_err(|e| self.err(e))?;
        let rgb = ops::extract_band_with_opts(img, 0, &ops::ExtractBandOptions { n: 3 })
            .map_err(|e| self.err(e))?;
        let img = ops::bandjoin(&mut [rgb, a]).map_err(|e| self.err(e))?;
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

    pub fn scale_to(&self, img: &VipsImage, w: Option<i32>, h: Option<i32>) -> Result<VipsImage> {
        let (iw, ih) = (img.get_width() as f64, img.get_height() as f64);
        let (sx, sy) = match (w, h) {
            (Some(rw), Some(rh)) => (rw as f64 / iw, rh as f64 / ih),
            (Some(rw), None) => {
                let s = rw as f64 / iw;
                (s, s)
            }
            (None, Some(rh)) => {
                let s = rh as f64 / ih;
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
        ox: Origin,
        oy: Origin,
    ) -> Result<(VipsImage, f64, f64)> {
        let (w, h) = (img.get_width() as f64, img.get_height() as f64);
        let ox = ox.apply(w);
        let oy = oy.apply(h);
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
        let Stroke { size, color } = stroke;
        let mask = ops::black(size * 2 + 1, size * 2 + 1).map_err(|e| self.err(e))?;
        let mask = ops::add(
            &mask,
            &VipsImage::new_from_image1(&mask, 128.0).map_err(|e| self.err(e))?,
        )
        .map_err(|e| self.err(e))?;
        ops::draw_circle_with_opts(
            &mask,
            &mut [255.0],
            size,
            size,
            size,
            &ops::DrawCircleOptions { fill: true },
        )
        .map_err(|e| self.err(e))?;

        let (w, h) = (img.get_width(), img.get_height());
        let img =
            ops::embed(&img, size, size, w + 2 * size, h + 2 * size).map_err(|e| self.err(e))?;

        let alpha = ops::extract_band(&img, 3).map_err(|e| self.err(e))?;
        // TODO: "binarize" alpha before blur for better results
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
        ox: Origin,
        oy: Origin,
        mode: BlendMode,
    ) -> Result<VipsImage> {
        let (bw, bh) = (base.get_width(), base.get_height());
        let (w, h) = (src.get_width() as f64, src.get_height() as f64);
        let ox = ox.apply(w) as i32;
        let oy = oy.apply(h) as i32;
        let src = ops::embed(&src, x - ox, y - oy, bw, bh).map_err(|e| self.err(e))?;
        ops::composite_2(&base, &src, mode.into()).map_err(|e| self.err(e))
    }

    pub fn print(
        &self,
        markup: Markup,
        im: &ImageMap,
        fm: &FontMap,
        font: &str,
        size: f64,
        color: Color,
        params: &[LayoutAttr],
    ) -> Result<(VipsImage, pango::Layout)> {
        if fm.get(font).is_none() {
            return Err(Error::font_missing(font));
        }
        let ctx = pangocairo::FontMap::new().create_context();
        let layout = pango::Layout::new(&ctx);
        params.iter().for_each(|p| p.configure(&ctx, &layout));

        let mut opt = cairo::FontOptions::new().map_err(Error::cairo)?;
        opt.set_antialias(cairo::Antialias::Good);
        pangocairo::functions::context_set_font_options(&ctx, Some(&opt));

        let gravity = Gravity::from(ctx.gravity());
        let (attrs, text) =
            markup.parsed(font.to_string(), pango::SCALE * size as i32, color, gravity);
        let (attr_list, images) = ITagAttr::vec_to_pango(attrs, self, im, fm, &ctx)?;
        layout.set_font_description(fm.get_desc_pt(font, size).as_ref());
        layout.set_attributes(Some(&attr_list));
        layout.set_text(&text);

        let (_, log_rect) = layout.extents();
        let mut base = {
            let base = cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                log_rect.width() / pango::SCALE,
                log_rect.height() / pango::SCALE,
            )
            .map_err(Error::cairo)?;
            let cr = cairo::Context::new(&base).map_err(Error::cairo)?;
            let (r, g, b, a) = color.rgba();
            cr.set_source_rgba(r, g, b, a);
            pangocairo::functions::show_layout(&cr, &layout);
            self.cairo_to_vips(base)?
        };

        if let Some(atl) = attr_list.filter(|att| att.type_() == pango::AttrType::Shape) {
            for (att, img) in atl.attributes().iter().zip(images) {
                if let Some(img) = img {
                    let i = att.start_index();
                    let rect = layout.index_to_pos(i as i32);
                    let (x, y) = (rect.x() / pango::SCALE, rect.y() / pango::SCALE);
                    base = self.overlay(
                        &base,
                        &img,
                        x,
                        y,
                        Origin::Absolute(0.0),
                        Origin::Absolute(0.0),
                        BlendMode::Over,
                    )?;
                }
            }
        }
        Ok((base, layout))
    }

    pub fn write(&self, img: &VipsImage, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_string_lossy();
        img.image_write_to_file(&path).map_err(|e| self.err(e))
    }
}
