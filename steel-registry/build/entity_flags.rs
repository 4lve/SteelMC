use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;

#[derive(Deserialize)]
struct EntityFlag {
    name: String,
    bit: u8,
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/entity_flags.json");

    let content = fs::read_to_string("build_assets/entity_flags.json").unwrap();
    let flags: Vec<EntityFlag> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse entity_flags.json: {}", e));

    let mut stream = TokenStream::new();

    for flag in &flags {
        let const_name = Ident::new(&flag.name.to_shouty_snake_case(), Span::call_site());
        let bit = flag.bit;

        stream.extend(quote! {
            pub const #const_name: u8 = #bit;
        });
    }

    stream
}
