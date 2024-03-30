extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Card)]
pub fn card(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(input);
    expand_derive_card(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_derive_card(ast: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let id_method = expand_derive_card_id(ast)?;
    let name = &ast.ident;
    let gen = quote! {
        impl Card for #name {
            #id_method
        }
    };
    Ok(gen)
}

fn expand_derive_card_id(_ast: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    // let field = match &ast.data {
    //     Data::Struct(struct_data) => {
    //         match &struct_data.fields {
    //             Fields::Unnamed(fields) => {
    //                 if let Some(field) = fields.unnamed.first() {
    //                     Ok(field)
    //                 } else {
    //                     Err(syn::Error::new(fields.span(), "expected struct with fields"))
    //                 }
    //             }
    //             Fields::Named(fields) => {
    //                 if let Some(field) = fields.named.iter().find(|f| f.ident.as_ref().map_or(false, |id| id == "id")) {
    //                     Ok(field)
    //                 } else {
    //                     Err(syn::Error::new(fields.span(), "expected struct with field named `id`"))
    //                 }
    //             }
    //             _ => Err(syn::Error::new(ast.span(), "expected struct with fields"))
    //         }
    //     }
    //     _ => Err(syn::Error::new(ast.span(), "expected struct with fields"))
    // }?;
    let gen = quote! {
        fn id(&self) -> Option<String> {
            Some(self.id.to_string())
        }
    };
    Ok(gen)
}