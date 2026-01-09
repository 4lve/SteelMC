use std::fs;

use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;

#[derive(Deserialize)]
struct SerializerEntry {
    name: String,
    id: i32,
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/entity_data_serializers.json");

    let file = "build_assets/entity_data_serializers.json";
    let content = fs::read_to_string(file).unwrap();
    let serializers: Vec<SerializerEntry> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse entity_data_serializers.json: {}", e));

    let mut stream = TokenStream::new();

    // Generate the EntityDataSerializer enum
    let mut variants = TokenStream::new();
    let mut from_id_arms = TokenStream::new();
    let mut to_id_arms = TokenStream::new();
    let mut name_arms = TokenStream::new();

    for serializer in &serializers {
        let variant_ident = Ident::new(&serializer.name.to_upper_camel_case(), Span::call_site());
        let id = serializer.id;
        let name = &serializer.name;

        variants.extend(quote! {
            #variant_ident,
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(i32)]
        pub enum EntityDataSerializer {
            #variants
        }

        impl EntityDataSerializer {
            #[must_use]
            pub const fn from_id(id: i32) -> Option<Self> {
                match id {
                    #from_id_arms
                    _ => None,
                }
            }

            #[must_use]
            pub const fn id(self) -> i32 {
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
