//! Implementation for the dynamic decoder, using Lua scripts.

use crate::abox::AliasBox;
use crate::cli::DynCard;
use crate::decode::Decoder;
use crate::error::{Error, Result};
use crate::layer::{ArtworkLayer, AssetLayer, LabelLayer, TextLayer};
use crate::layer::{Layer, LayerStack};

use mlua::{
    Error as LuaError, FromLua, Function, Lua, Result as LuaResult, Table, UserData,
    Value as LuaValue, Variadic,
};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LuaDecoderFactory {
    folder: PathBuf,
    chunk: String,
}

impl LuaDecoderFactory {
    pub fn new(folder: PathBuf) -> Result<Self> {
        let mut path = folder.clone();
        path.push("decode.lua");
        let chunk = fs::read_to_string(&path)
            .map_err(|e| Error::decoder_open(path, e))?;
        Ok(Self { folder, chunk })
    }

    pub fn create(&self) -> Result<LuaDecoder> {
        LuaDecoder::new(&self.folder, &self.chunk)
    }
}

pub struct LuaDecoder {
    // actually has lifetime of `_lua``
    decode: Function<'static>,
    // SAFETY: we must never move out of this box as long as `decode` is alive
    _lua: AliasBox<Lua>,
}

macro_rules! register {
    (($( $layer:ty ),*) to $lua:expr, $module:expr) => {
        $(
            <$layer>::register($lua, $module)?;
        )*
    }
}

impl LuaDecoder {
    fn new(req_path: &PathBuf, chunk: &str) -> Result<Self> {
        let lua = AliasBox::new(Lua::new());

        Self::create_layer_module(&lua).map_err(Error::decoder_prep)?;

        Self::extend_package_path(&lua, req_path.display().to_string().as_str())
            .map_err(Error::decoder_prep)?;

        let decode: Function = lua
            .load(chunk)
            .call(())
            .map_err(Error::decoder_prep)?;

        let decode = unsafe { std::mem::transmute(decode) };

        Ok(Self {
            decode,
            _lua: lua,
        })
    }

    fn extend_package_path(lua: &Lua, req_path: &str) -> LuaResult<()> {
        let globals = &lua.globals();
        let package: Table = globals.get("package")?;
        let path: String = package.get("path")?;
        package.set(
            "path",
            format!("{}/?.lua;{}/?/init.lua;{path}", req_path, req_path),
        )?;
        let cpath: String = package.get("cpath")?;
        package.set("cpath", format!("{}/?.so;{cpath}", &req_path))?;
        Ok(())
    }

    fn create_layer_module(lua: &Lua) -> LuaResult<()> {
        let globals = &lua.globals();
        let loaded: Table = globals
            .get::<_, Table>("package")?
            .get::<_, Table>("loaded")?;
        let module = match loaded.get("cartomata.layer")? {
            LuaValue::Table(module) => Ok(module),
            LuaValue::Nil => {
                let module = lua.create_table()?;
                loaded.set("cartomata.layer", &module)?;
                Ok(module)
            }
            _ => Err(LuaError::RuntimeError(
                "failed to create cartomata.layer module".to_string(),
            )),
        }?;
        register!((ArtworkLayer, AssetLayer, LabelLayer, TextLayer) to &lua, &module);
        Ok(())
    }
}

macro_rules! cast_layer {
    (($value:expr, $lua:expr, $layer:expr) to $($ltype:ty)|*) => {
        {
            $(if $layer.is::<$ltype>() {
                <$ltype>::from_lua($value, $lua).map(|l| Box::new(l) as Box<dyn Layer>)
            }) else *
            else {
                Err(LuaError::FromLuaConversionError {
                    from: $value.type_name(),
                    to: "Layer",
                    message: None,
                })
            }
        }
    };
}

impl UserData for Box<dyn Layer> {}

impl<'lua> FromLua<'lua> for Box<dyn Layer> {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match &value {
            LuaValue::UserData(ud) => {
                cast_layer!(
                    (value, lua, ud)
                    to AssetLayer | ArtworkLayer | LabelLayer | TextLayer
                )
            }
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Layer",
                message: None,
            }),
        }
    }
}

impl Decoder<DynCard> for LuaDecoder {
    fn decode(&self, card: &DynCard) -> Result<LayerStack> {
        let layers: Variadic<Box<dyn Layer>> = self
            .decode
            .call(card.0.clone())
            .map_err(Error::decode)?;
        Ok(LayerStack(layers.into_iter().collect()))
    }
}
