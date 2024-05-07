extern crate proc_macro;

mod expand;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Card)]
pub fn card(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    expand::derive_card(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(LuaLayer)]
pub fn lua_layer(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    expand::derive_lua_layer(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
