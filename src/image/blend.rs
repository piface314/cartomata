//! Image blend modes.

use libvips::ops;
#[cfg(feature = "cli")]
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
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "cli", derive(Deserialize, Serialize))]
    #[cfg_attr(feature = "cli", serde(rename_all = "kebab-case"))]
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
        Exclusion,
        Last
    }
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Over
    }
}
