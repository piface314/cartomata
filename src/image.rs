//! Image backend implementations.

mod blend;
mod blur;
mod color;
mod map;
mod origin;
mod stroke;

use crate::error::{ImgError, TextError};
pub use crate::image::blur::Blur;
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

type ImgResult<T> = Result<T, ImgError>;

impl From<VipsApp> for ImgBackend {
    fn from(vips_app: VipsApp) -> Self {
        Self { vips_app }
    }
}

impl ImgBackend {
    pub fn new() -> ImgResult<Self> {
        let vips_app = libvips::VipsApp::default("cartomata")
            .map_err(|source| ImgError::Vips { source, details: None })?;
        Ok(Self { vips_app })
    }

    pub fn err(&self, source: libvips::error::Error) -> ImgError {
        ImgError::Vips {
            source,
            details: self.vips_app.error_buffer().ok().map(|s| s.to_string()),
        }
    }

    pub fn reinterpret(&self, img: &VipsImage) -> ImgResult<VipsImage> {
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

    pub fn create(&self, bg: &Color, width: i32, height: i32) -> ImgResult<VipsImage> {
        self.reinterpret(&self.create_raw(bg, width, height)?)
    }

    pub fn create_raw(&self, bg: &Color, width: i32, height: i32) -> ImgResult<VipsImage> {
        if bg.has_alpha() {
            let (r, g, b, a) = bg.scaled_rgba();
            let img = ops::black_with_opts(width, height, &ops::BlackOptions { bands: 4 })
                .map_err(|e| self.err(e))?;
            let img = VipsImage::new_from_image(&img, &[r, g, b, a]).map_err(|e| self.err(e))?;
            Ok(img)
        } else {
            let (r, g, b) = bg.scaled_rgb();
            let img = ops::black_with_opts(width, height, &ops::BlackOptions { bands: 3 })
                .map_err(|e| self.err(e))?;
            let img = VipsImage::new_from_image(&img, &[r, g, b]).map_err(|e| self.err(e))?;
            Ok(img)
        }
    }

    pub fn cairo_to_vips(&self, img: ImageSurface) -> ImgResult<VipsImage> {
        let mut buffer = Vec::new();
        img.write_to_png(&mut buffer)
            .map_err(ImgError::cairo_to_vips)?;
        let mut img = VipsImage::new_from_buffer(&buffer, "").map_err(|e| self.err(e))?;
        img.image_wio_input().map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    pub fn open(&self, fp: impl AsRef<str>) -> ImgResult<VipsImage> {
        self.reinterpret(&self.open_raw(fp.as_ref())?)
    }

    pub fn open_raw(&self, fp: impl AsRef<str>) -> ImgResult<VipsImage> {
        let fp = fp.as_ref();
        let img = VipsImage::new_from_file(fp).map_err(|e| self.err(e))?;
        Ok(img)
    }

    pub fn set_color(&self, img: &VipsImage, color: Color) -> ImgResult<VipsImage> {
        let (r, g, b) = color.scaled_rgb();
        let rgb = VipsImage::new_from_image(img, &[r, g, b]).map_err(|e| self.err(e))?;
        let img_a = ops::extract_band(&img, 3).map_err(|e| self.err(e))?;
        let out_a = match color.a {
            Some(a) => self.set_opacity_checked(img_a, a)?,
            None => img_a,
        };
        let img = ops::bandjoin(&mut [rgb, out_a]).map_err(|e| self.err(e))?;
        self.reinterpret(&img)
    }

    /// Multiplies the image opacity by the given value.
    pub fn set_opacity(&self, img: &VipsImage, alpha: f64) -> ImgResult<VipsImage> {
        let n_bands = img.get_bands();
        let img_a = ops::extract_band(img, n_bands - 1).map_err(|e| self.err(e))?;
        let img_a = self.uchar_to_double(&img_a)?;
        let new_a = VipsImage::new_from_image1(&img_a, alpha).map_err(|e| self.err(e))?;
        let out_a = ops::multiply(&img_a, &new_a).map_err(|e| self.err(e))?;
        let out_a = self.double_to_uchar(&out_a)?;
        if n_bands > 1 {
            let rgb =
                ops::extract_band_with_opts(img, 0, &ops::ExtractBandOptions { n: n_bands - 1 })
                    .map_err(|e| self.err(e))?;
            let img = ops::bandjoin(&mut [rgb, out_a]).map_err(|e| self.err(e))?;
            self.reinterpret(&img)
        } else {
            Ok(out_a)
        }
    }

    /// Multiplies the image opacity by the given value.
    ///
    /// If alpha is `1.0`, the image is returned unchanged.
    pub fn set_opacity_checked(&self, img: VipsImage, alpha: f64) -> ImgResult<VipsImage> {
        if alpha.abs() - 1.0 < f64::EPSILON {
            Ok(img)
        } else {
            self.set_opacity(&img, alpha)
        }
    }

    /// Crops the image, extracting the specified area.
    pub fn crop(
        &self,
        img: &VipsImage,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        ox: Origin,
        oy: Origin,
    ) -> ImgResult<VipsImage> {
        let (ox, oy) = (ox.apply(w as f64).round(), oy.apply(h as f64).round());
        let (x, y) = (
            (x as f64 - ox).round() as i32,
            (y as f64 - oy).round() as i32,
        );
        let out = ops::extract_area(img, x, y, w, h).map_err(|e| self.err(e))?;
        Ok(out)
    }

    /// Casts a [`libvips::ops::BandFormat::Uchar`] image to [`libvips::ops::BandFormat::Double`],
    /// scaling its values from the `[0.0, 1.0]` range to the `[0, 255]` range.
    pub fn uchar_to_double(&self, img: &VipsImage) -> ImgResult<VipsImage> {
        let out = ops::cast(img, ops::BandFormat::Double).map_err(|e| self.err(e))?;
        let const255 = &VipsImage::new_from_image1(&out, 255.0).map_err(|e| self.err(e))?;
        let out = ops::divide(&out, &const255).map_err(|e| self.err(e))?;
        Ok(out)
    }

    /// Casts a [`libvips::ops::BandFormat::Double`] image to [`libvips::ops::BandFormat::Uchar`],
    /// scaling its values from the `[0, 255]` range to the `[0.0, 1.0]` range.
    pub fn double_to_uchar(&self, img: &VipsImage) -> ImgResult<VipsImage> {
        let const255 = &VipsImage::new_from_image1(&img, 255.0).map_err(|e| self.err(e))?;
        let out = ops::multiply(&img, &const255).map_err(|e| self.err(e))?;
        let out = ops::cast(&out, ops::BandFormat::Uchar).map_err(|e| self.err(e))?;
        Ok(out)
    }

    /// Scales the image by the given horizontal and vertical factors.
    pub fn scale(&self, img: &VipsImage, sx: f64, sy: f64) -> ImgResult<VipsImage> {
        ops::resize_with_opts(
            &img,
            sx,
            &ops::ResizeOptions { vscale: sy, ..Default::default() },
        )
        .map_err(|e| self.err(e))
    }

    /// Scales the image by the given horizontal and vertical factors.
    ///
    /// If both `sx` and `sy` are `1.0`, the image is returned unchanged.
    pub fn scale_checked(&self, img: VipsImage, sx: f64, sy: f64) -> ImgResult<VipsImage> {
        if sx.abs() - 1.0 < f64::EPSILON && sy.abs() - 1.0 < f64::EPSILON {
            Ok(img)
        } else {
            self.scale(&img, sx, sy)
        }
    }

    /// Scales the image to the given size.
    pub fn scale_to(
        &self,
        img: &VipsImage,
        w: Option<i32>,
        h: Option<i32>,
    ) -> ImgResult<VipsImage> {
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

    /// Scales the image to the given size.
    ///
    /// If both width and height are [`None`], the image is returned unchanged.
    pub fn scale_to_checked(
        &self,
        img: VipsImage,
        w: Option<i32>,
        h: Option<i32>,
    ) -> ImgResult<VipsImage> {
        if w.is_none() && h.is_none() {
            Ok(img)
        } else {
            self.scale_to(&img, w, h)
        }
    }

    /// Scales the image so that it fits inside the given area, according to the [`FitMode`].
    pub fn scale_to_fit(
        &self,
        img: &VipsImage,
        w: f64,
        h: f64,
        mode: FitMode,
    ) -> ImgResult<VipsImage> {
        let (iw, ih) = (img.get_width() as f64, img.get_height() as f64);
        let aspect_ratio = iw / ih;
        let (sx, sy) = match mode {
            FitMode::Contain | FitMode::Cover => {
                if (aspect_ratio < 1.0) ^ (mode == FitMode::Contain) {
                    let s = w / iw;
                    (s, s)
                } else {
                    let s = h / ih;
                    (s, s)
                }
            }
            FitMode::Stretch => (1.0, 1.0),
        };
        self.scale(img, sx, sy)
    }

    /// Rotates the image around the given origin point.
    /// The horizontal and vetical displacements are also returned.
    pub fn rotate(
        &self,
        img: &VipsImage,
        deg: f64,
        ox: Origin,
        oy: Origin,
    ) -> Result<(VipsImage, f64, f64), ImgError> {
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

    /// Rotates the image around the given origin point.
    /// The horizontal and vetical displacements are also returned.
    ///
    /// If `deg` is zero, the image is returned unchanged.
    pub fn rotate_checked(
        &self,
        img: VipsImage,
        deg: f64,
        ox: Origin,
        oy: Origin,
    ) -> Result<(VipsImage, f64, f64), ImgError> {
        if deg < f64::EPSILON {
            let (w, h) = (img.get_width() as f64, img.get_height() as f64);
            let ox = ox.apply(w);
            let oy = oy.apply(h);
            Ok((img, ox, oy))
        } else {
            self.rotate(&img, deg, ox, oy)
        }
    }

    /// Draws a stroke around the visible edges of the image.
    pub fn stroke(&self, img: &VipsImage, stroke: Stroke) -> ImgResult<VipsImage> {
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
            &ops::GaussblurOptions { min_ampl: 0.2, ..Default::default() },
        )
        .map_err(|e| self.err(e))?;

        let (r, g, b) = color.scaled_rgb();
        let stroke = VipsImage::new_from_image(&alpha, &[r, g, b]).map_err(|e| self.err(e))?;
        let stroke = ops::bandjoin(&mut [stroke, alpha]).map_err(|e| self.err(e))?;
        let stroke = self.reinterpret(&stroke)?;
        let img = ops::composite2(&stroke, &img, ops::BlendMode::Over).map_err(|e| self.err(e))?;
        Ok(img)
    }

    /// Applies gaussian blur to the image.
    pub fn blur(&self, img: &VipsImage, sigma: f64) -> ImgResult<(VipsImage, f64, f64)> {
        let (w, h) = (img.get_width() as f64, img.get_height() as f64);
        let out = ops::gaussblur(img, sigma).map_err(|e| self.err(e))?;
        let (tw, th) = (out.get_width() as f64, out.get_height() as f64);
        Ok((out, (tw - w) / 2.0, (th - h) / 2.0))
    }

    /// Applies gaussian blur to the image.
    /// 
    /// If `sigma` is zero, the image is returned unchanged.
    pub fn blur_checked(&self, img: VipsImage, sigma: f64) -> ImgResult<(VipsImage, f64, f64)> {
        if sigma.abs() < f64::EPSILON {
            Ok((img, 0.0, 0.0))
        } else {
            self.blur(&img, sigma)
        }
    }

    /// Draws a stroke around the visible edges of the image.
    ///
    /// If the stroke size is zero, the image is returned unchanged.
    pub fn stroke_checked(&self, img: VipsImage, stroke: Stroke) -> ImgResult<VipsImage> {
        if stroke.size == 0 {
            Ok(img)
        } else {
            self.stroke(&img, stroke)
        }
    }

    /// Rounds the four corners of the given image using the given radius.
    #[cfg(feature = "vips-8-16")]
    pub fn round_corners(&self, img: &VipsImage, radius: i32) -> ImgResult<VipsImage> {
        let (w, h) = (img.get_width(), img.get_height());
        let min_dim = w.min(h);
        let r = if radius * 2 > min_dim {
            (min_dim as f64 / 2.0).ceil()
        } else {
            radius as f64
        };
        let sdf = ops::sdf_with_opts(
            w,
            h,
            ops::SdfShape::RoundedBox,
            &ops::SdfOptions {
                a: vec![0.0, 0.0],
                b: vec![w as f64, h as f64],
                corners: vec![r, r, r, r],
                ..Default::default()
            },
        )
        .map_err(|e| self.err(e))?;
        let const1 = VipsImage::new_from_image1(&sdf, 1.0).map_err(|e| self.err(e))?;
        let mask = ops::subtract(&const1, &sdf).map_err(|e| self.err(e))?;
        self.mask_alpha(img, &mask)
    }

    /// Rounds the four corners of the given image using the given radius.
    ///
    /// If the radius is zero, the image is returned unchanged.
    #[cfg(feature = "vips-8-16")]
    pub fn round_corners_checked(&self, img: VipsImage, radius: i32) -> ImgResult<VipsImage> {
        if radius == 0 {
            Ok(img)
        } else {
            self.round_corners(&img, radius)
        }
    }

    /// Applies a mask to the given image alpha channel.
    ///
    /// `mask` is expected to already be using the double format, in the `[0.0, 1.0]` range.
    pub fn mask_alpha(&self, img: &VipsImage, mask: &VipsImage) -> ImgResult<VipsImage> {
        let mask_bands = mask.get_bands();
        let mask = ops::extract_band(mask, mask_bands - 1).map_err(|e| self.err(e))?;
        let img_bands = img.get_bands();
        let img_a = ops::extract_band(img, img_bands - 1).map_err(|e| self.err(e))?;
        let img_a = self.uchar_to_double(&img_a)?;
        let out_a = ops::multiply(&img_a, &mask).map_err(|e| self.err(e))?;
        let out_a = self.double_to_uchar(&out_a)?;
        if img_bands > 1 {
            let img_rgb =
                ops::extract_band_with_opts(img, 0, &ops::ExtractBandOptions { n: img_bands - 1 })
                    .map_err(|e| self.err(e))?;
            let out = ops::bandjoin(&mut [img_rgb, out_a]).map_err(|e| self.err(e))?;
            Ok(out)
        } else {
            Ok(out_a)
        }
    }

    pub fn overlay(
        &self,
        base: &VipsImage,
        src: &VipsImage,
        x: i32,
        y: i32,
        mode: BlendMode,
    ) -> ImgResult<VipsImage> {
        let (bw, bh) = (base.get_width(), base.get_height());
        let src = ops::embed(&src, x, y, bw, bh).map_err(|e| self.err(e))?;
        ops::composite2(&base, &src, mode.into()).map_err(|e| self.err(e))
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
    ) -> Result<(VipsImage, pango::Layout), ImgError> {
        if fm.get(font).is_none() {
            return Err(TextError::font_missing(font).into());
        }
        let ctx = pangocairo::FontMap::new().create_context();
        let layout = pango::Layout::new(&ctx);
        params.iter().for_each(|p| p.configure(&ctx, &layout));

        let mut opt = cairo::FontOptions::new()?;
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
            )?;
            let cr = cairo::Context::new(&base)?;
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
                    base = self.overlay(&base, &img, x, y, BlendMode::Over)?;
                }
            }
        }
        Ok((base, layout))
    }

    pub fn write(&self, img: &VipsImage, path: impl AsRef<Path>) -> Result<(), ImgError> {
        let path = path.as_ref().to_string_lossy();
        img.image_write_to_file(&path).map_err(|e| self.err(e))
    }
}
