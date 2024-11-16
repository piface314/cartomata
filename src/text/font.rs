//! Management of font files and configuration

use crate::error::{Error, Result};

use fontconfig::{Fontconfig, Pattern};
use fontconfig_sys::fontconfig as sys;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, ffi::CString};

#[derive(Debug, Clone)]
pub enum FontPath {
    Path(PathBuf),
    Desc { name: String, style: Option<String> },
}

pub struct FontMap {
    fc: Fontconfig,
    loaded: HashMap<String, String>,
}

impl std::fmt::Debug for FontMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FontMap({:?})", self.loaded)
    }
}

impl FontMap {
    pub fn new() -> Result<Self> {
        Ok(Self {
            fc: fontconfig::Fontconfig::new().ok_or(Error::FontMapInit)?,
            loaded: HashMap::new(),
        })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.loaded.get(key).map(|s| s.as_str())
    }

    pub fn get_desc(&self, key: &str) -> Option<pango::FontDescription> {
        self.get(key)
            .map(|name| pango::FontDescription::from_string(name))
    }

    pub fn get_desc_pt(&self, key: &str, size: f64) -> Option<pango::FontDescription> {
        self.get(key)
            .map(|name| pango::FontDescription::from_string(&format!("{} {size:.2}", name)))
    }

    pub fn get_desc_abs(&self, key: &str, size: i32) -> Option<pango::FontDescription> {
        self.get_desc_pt(key, size as f64 / pango::SCALE as f64)
    }

    pub fn load(&mut self, fonts: HashMap<String, FontPath>) -> Result<()> {
        for (key, cfg) in fonts.into_iter() {
            match cfg {
                FontPath::Desc { name, style } => {
                    self.load_font_from_name(key, &name, style.as_ref().map(|s| s.as_str()))?
                }
                FontPath::Path(fp) => self.load_font_from_file(key, fp)?,
            };
        }
        Ok(())
    }

    pub fn load_font_from_name(
        &mut self,
        key: String,
        family: &str,
        style: Option<&str>,
    ) -> Result<()> {
        let mut pat = Pattern::new(&self.fc);
        let c_family =
            CString::new(family).map_err(|_| Error::font_load(&key, "family", family))?;
        pat.add_string(sys::constants::FC_FAMILY.as_cstr(), &c_family);

        if let Some(style) = style {
            let c_style =
                CString::new(style).map_err(|_| Error::font_load(&key, "style", style))?;
            pat.add_string(sys::constants::FC_STYLE.as_cstr(), &c_style);
        }

        let name = pat
            .font_match()
            .name()
            .ok_or_else(|| Error::font_unnamed(&key))?
            .to_string();
        self.loaded.insert(key, name);
        Ok(())
    }

    pub fn load_font_from_file(&mut self, key: String, path: impl AsRef<Path>) -> Result<()> {
        let fp = path.as_ref().to_string_lossy();
        let c_fp = CString::new(fp.as_bytes()).map_err(|_| Error::font_load(&key, "path", fp))?;
        let mut pat = self
            .load_pattern_from_file(&c_fp)
            .ok_or_else(|| Error::font_file_load(&key, &path))?;

        let status = unsafe {
            fontconfig_sys::fontconfig::FcConfigAppFontAddFile(
                std::ptr::null_mut(),
                c_fp.as_ptr() as *const sys::FcChar8,
            )
        };
        if status == 0 {
            Err(Error::font_file_load(&key, &path))
        } else {
            let name = pat
                .font_match()
                .name()
                .ok_or_else(|| Error::font_unnamed(&key))?
                .to_string();
            drop(pat);
            self.loaded.insert(key, name);
            Ok(())
        }
    }

    fn load_pattern_from_file<'s>(&'s self, c_fp: &CString) -> Option<Pattern<'s>> {
        unsafe {
            let set = sys::FcFontSetCreate();
            let status = sys::FcFileScan(
                set,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                c_fp.as_ptr() as *const sys::FcChar8,
                1,
            );
            let result = if status == 0 || (*set).nfont < 1 {
                None
            } else {
                let pat_ptr = *(*set).fonts;
                let pat = Pattern::from_pattern(&self.fc, pat_ptr);
                Some(pat)
            };
            sys::FcFontSetDestroy(set);
            result
        }
    }
}
