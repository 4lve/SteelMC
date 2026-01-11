use std::fs;

use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;

#[derive(Deserialize)]
struct PoseEntry {
    name: String,
    id: i32,
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/poses.json");

    let file = "build_assets/poses.json";
    let content = fs::read_to_string(file).unwrap();
    let poses: Vec<PoseEntry> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse poses.json: {}", e));

    let mut stream = TokenStream::new();
    let mut variants = TokenStream::new();
    let mut from_id_arms = TokenStream::new();
    let mut to_id_arms = TokenStream::new();
    let mut name_arms = TokenStream::new();

    for pose in &poses {
        let variant_ident = Ident::new(&pose.name.to_upper_camel_case(), Span::call_site());
        let id = pose.id as u8;
        let name = &pose.name;

        variants.extend(quote! {
            #variant_ident = #id,
        });

        from_id_arms.extend(quote! {
            #id => Some(Self::#variant_ident),
        });

        to_id_arms.extend(quote! {
            Self::#variant_ident => #id,
        });

        name_arms.extend(quote! {
            Self::#variant_ident => #name,
        });
    }

    stream.extend(quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        #[repr(u8)]
        pub enum Pose {
            #[default]
            #variants
        }

        impl Pose {
            #[must_use]
            pub const fn from_id(id: u8) -> Option<Self> {
                match id {
                    #from_id_arms
                    _ => None,
                }
            }

            #[must_use]
            pub const fn id(self) -> u8 {
                match self {
                    #to_id_arms
                }
            }

            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    #name_arms
                }
            }
        }
    });

    stream
}
