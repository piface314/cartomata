use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_card(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let id_method = derive_card_id(ast)?;
    let name = &ast.ident;
    let gen = quote! {
        impl Card for #name {
            #id_method
        }
    };
    Ok(gen)
}

pub fn derive_card_id(_ast: &DeriveInput) -> syn::Result<TokenStream> {
    let gen = quote! {
        fn id(&self) -> String {
            self.id.to_string()
        }
    };
    Ok(gen)
}

pub fn derive_lua_layer(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;

    let name_str = name.to_string();

    let gen = quote! {
        impl mlua::UserData for #name {}

        impl #name {
            pub fn register(lua: &mlua::Lua, module: &mlua::Table) -> mlua::Result<()> {
                let f = lua.create_function(|lua: &mlua::Lua, (params, ): (mlua::Value,)| {
                    let layer: #name = lua.from_value(params)?;
                    Ok(layer)
                })?;
                module.set(#name_str, f)?;
                Ok(())
            }
        }

        impl<'lua> mlua::FromLua<'lua> for #name {
            fn from_lua(value: mlua::Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
                match value {
                    mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
                    _ => Err(mlua::Error::FromLuaConversionError {
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
