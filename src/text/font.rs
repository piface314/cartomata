//! Management of font files and configuration

use crate::error::{Error, Result};
use crate::template::Template;

use fontconfig::{Fontconfig, Pattern};
use fontconfig_sys::fontconfig as sys;
use std::path::Path;
use std::{collections::HashMap, ffi::CString};

pub struct FontManager<'f> {
    fc: &'f Fontconfig,
    loaded: HashMap<&'f str, Pattern<'f>>,
}

impl<'f> FontManager<'f> {
    pub fn new(fc: &'f Fontconfig) -> Self {
        Self {
            fc,
            loaded: HashMap::new(),
        }
    }

    pub fn get(&'f self, key: &str) -> Option<&'f Pattern<'f>> {
        self.loaded.get(key)
    }

    pub fn get_desc(&self, key: &str, size: f64) -> Option<pango::FontDescription> {
        self.loaded.get(key).map(|pat| {
            pango::FontDescription::from_string(&format!("{} {size:.2}", pat.name().unwrap_or("")))
        })
    }

    pub fn check(&'f self, key: &str) -> Option<(&'f str, &'f Pattern<'f>)> {
        self.loaded.get_key_value(key).map(|(k, v)| (*k, v))
    }

    pub fn load_from_template(&mut self, template: &'f Template) -> Result<()> {
        for (key, cfg) in template.fonts.iter() {
            if let Some(fp) = &cfg.path {
                let mut path = template.folder()?;
                path.push(fp);
                self.load_font_from_file(key, path)?;
            } else if let Some(family) = &cfg.family {
                self.load_font_from_name(key, &family, cfg.style.as_deref())?;
            } else {
                return Err(Error::FontUndefined(key.to_string()));
            };
        }
        Ok(())
    }

    pub fn load_font_from_name(
        &mut self,
        key: &'f str,
        family: &str,
        style: Option<&str>,
    ) -> Result<&Pattern<'f>> {
        let mut pat = Pattern::new(self.fc);
        let c_family =
            CString::new(family).map_err(|_| Error::InvalidCString(family.to_string()))?;
        pat.add_string(sys::constants::FC_FAMILY.as_cstr(), &c_family);

        if let Some(style) = style {
            let c_style =
                CString::new(style).map_err(|_| Error::InvalidCString(style.to_string()))?;
            pat.add_string(sys::constants::FC_STYLE.as_cstr(), &c_style);
        }

        let pat = Pattern::from_pattern(self.fc, pat.font_match().pat);
        self.loaded.insert(key, pat);
        Ok(self.loaded.get(key).unwrap())
    }

    pub fn load_font_from_file(
        &mut self,
        key: &'f str,
        fp: impl AsRef<Path>,
    ) -> Result<&Pattern<'f>> {
        let fp = fp.as_ref();
        let c_fp = CString::new(fp.to_string_lossy().to_string())
            .map_err(|_| Error::InvalidCString(fp.to_string_lossy().to_string()))?;
        let pat = self
            .load_pattern_from_file(&c_fp)
            .ok_or_else(|| Error::LoadFontError(key.into()))?;

        let status = unsafe {
            fontconfig_sys::fontconfig::FcConfigAppFontAddFile(
                std::ptr::null_mut(),
                c_fp.as_ptr() as *const sys::FcChar8,
            )
        };
        if status == 0 {
            Err(Error::LoadFontError(key.into()))
        } else {
            self.loaded.insert(key, pat);
            Ok(self.loaded.get(key).unwrap())
        }
    }

    fn load_pattern_from_file(&self, c_fp: &CString) -> Option<Pattern<'f>> {
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
