//! Implementations to decode card data into layers.

use crate::data::{Card, DynCard};
use crate::error::{Error, Result};
use crate::layer::artwork::ArtworkLayer;
use crate::layer::asset::AssetLayer;
use crate::layer::{Layer, LayerStack};
use crate::template::Template;

use mlua::{
    Error as LuaError, FromLua, Function, Lua, Result as LuaResult, Table, UserData,
    Value as LuaValue, Variadic,
};
use std::fs;

pub trait Decoder<C: Card> {
    fn decode(&self, card: C) -> Result<LayerStack>;
}

pub struct DynamicDecoder<'lua> {
    decode: Function<'lua>,
}

macro_rules! register {
    ($lua:expr, $module:expr, $( $layer:ty )*) => {
        $(
            <$layer>::register($lua, $module)
            .map_err(|e| Error::FailedPrepareDecoder(e.to_string()))?;
        )*
    }
}

impl<'lua> DynamicDecoder<'lua> {
    pub fn new(lua: &'lua Lua, template: &Template) -> Result<Self> {
        let req_path = template.folder()?;
        let mut path = req_path.clone();
        path.push("decode.lua");

        let chunk = fs::read_to_string(&path)
            .map_err(|e| Error::FailedOpenDecoder(path.display().to_string(), e.to_string()))?;

        let module = Self::create_layer_module(lua)
            .map_err(|e| Error::FailedPrepareDecoder(e.to_string()))?;

        Self::extend_package_path(lua, req_path.display().to_string().as_str())
            .map_err(|e| Error::FailedPrepareDecoder(e.to_string()))?;

        register!(lua, &module, AssetLayer ArtworkLayer);

        let decode: Function = lua
            .load(&chunk)
            .call(())
            .map_err(|e| Error::FailedOpenDecoder(String::new(), e.to_string()))?;

        Ok(Self { decode })
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

    fn create_layer_module(lua: &Lua) -> LuaResult<Table> {
        let globals = &lua.globals();
        let loaded: Table = globals
            .get::<_, Table>("package")?
            .get::<_, Table>("loaded")?;
        match loaded.get("cartomata.layer")? {
            LuaValue::Table(module) => Ok(module),
            LuaValue::Nil => {
                let module = lua.create_table()?;
                loaded.set("cartomata.layer", &module)?;
                Ok(module)
            }
            _ => Err(LuaError::RuntimeError(
                "failed to create cartomata.layer module".to_string(),
            )),
        }
    }
}

macro_rules! cast_layer {
    ($value:expr, $lua:expr, $layer:expr, $($ltype:ty)*) => {
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
                cast_layer!(value, lua, ud, AssetLayer ArtworkLayer)
            }
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Layer",
                message: None,
            }),
        }
    }
}

impl<'lua> Decoder<DynCard> for DynamicDecoder<'lua> {
    fn decode(&self, card: DynCard) -> Result<LayerStack> {
        let DynCard(card_data) = card;
        let layers: Variadic<Box<dyn Layer>> = self
            .decode
            .call(card_data)
            .map_err(|e| Error::Decoding(e.to_string()))?;
        Ok(LayerStack(layers.into_iter().collect()))
    }
}
