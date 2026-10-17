//! Image blend modes.

use crate::error::ImgError;
use crate::layer::{LayerFx, LayerOut, RenderContext};
use libvips::ops;
use serde::{Deserialize, Serialize};

macro_rules! into_vips {
    (
        $(#[$outer:meta])*
        $vis:vis enum $Enum:ident {
            $( $Variant:ident ),*
        }
    ) => {
        $(#[$outer])*
        $vis enum $Enum {
            $( $Variant ),*
        }

        impl Into<ops::$Enum> for $Enum {
            fn into(self) -> ops::$Enum {
                match self {
                    $( Self::$Variant => ops::$Enum::$Variant ),*
                }
            }
        }
    };
}

into_vips! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum BlendMode {
        Clear,
        Source,
        Over,
        In,
        Out,
        Atop,
        Dest,
        DestOver,
        DestIn,
        DestOut,
        DestAtop,
        Xor,
        Add,
        Saturate,
        Multiply,
        Screen,
        Overlay,
        Darken,
        Lighten,
        ColourDodge,
        ColourBurn,
        HardLight,
        SoftLight,
        Difference,
        Exclusion
    }
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Over
    }
}

impl From<BlendMode> for LayerFx {
    fn from(blend: BlendMode) -> Self {
        let desc = format!("{blend:?}");
        Self::new(
            desc,
            move |_: &RenderContext, out: LayerOut| -> Result<LayerOut, ImgError> {
                Ok(out.with_blend_mode(blend))
            },
        )
    }
}
