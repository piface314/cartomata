use proc_macro2::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, spanned::Spanned, Data, DataStruct, DeriveInput, Fields};

pub fn derive_predicate_access(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let idents = match &ast.data {
        Data::Struct(DataStruct { fields: Fields::Named(fields), .. }) => Ok(fields
            .named
            .iter()
            .map(|field| field.ident.as_ref().unwrap())),
        _ => Err(syn::Error::new(
            ast.span(),
            "expected struct with named fields",
        )),
    }?;
    let arms = idents.map(|ident| {
        let unraw_ident = ident.unraw();
        quote! {
            stringify!(#unraw_ident) => ::cartomata::data::Access::access(&self.#ident, rest)
        }
    });
    let gen = quote! {
        impl ::cartomata::data::Access for #name {
            fn access(&'_ self, parts: &[::cartomata::data::PredicatePathPart]) -> ::cartomata::data::ValueRef<'_> {
                match parts {
                    [
                        ::cartomata::data::PredicatePathPart::Key(key),
                        rest @ ..
                    ] => {
                        match key.as_str() {
                            #(#arms),*,
                            _ => ::cartomata::data::ValueRef::Null
                        }
                    }
                    _ => ::cartomata::data::ValueRef::Null
                }
            }
        }
    };
    Ok(gen)
}

pub fn derive_lua_layer(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;

    let name_str = name.to_string();

    let gen = quote! {
        impl ::mlua::UserData for #name {}

        impl #name {
            pub fn register(lua: &::mlua::Lua, module: &::mlua::Table) -> ::mlua::Result<()> {
                let f = lua.create_function(|lua: &::mlua::Lua, (params, ): (::mlua::Value,)| {
                    let layer: #name = lua.from_value(params)?;
                    Ok(layer)
                })?;
                module.set(#name_str, f)?;
                Ok(())
            }
        }

        impl<'lua> ::mlua::FromLua<'lua> for #name {
            fn from_lua(value: ::mlua::Value<'lua>, _: &'lua ::mlua::Lua) -> ::mlua::Result<Self> {
                match value {
                    ::mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
                    _ => Err(::mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: #name_str,
                        message: None,
                    })
                }
            }
        }
    };
    Ok(gen)
}
