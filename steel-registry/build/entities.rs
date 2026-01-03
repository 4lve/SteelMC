use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Entities {
    #[allow(dead_code)]
    poses: Vec<PoseEntry>,
    entity_data_serializers: Vec<SerializerEntry>,
    entity_data_accessors: EntityDataAccessors,
    entity_types: Vec<EntityTypeEntry>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct PoseEntry {
    name: String,
    id: u8,
    serialized_name: String,
}

#[derive(Deserialize)]
struct SerializerEntry {
    name: String,
    id: u8,
}

#[derive(Deserialize)]
struct EntityDataAccessors {
    entity: Vec<AccessorEntry>,
}

#[derive(Deserialize)]
struct AccessorEntry {
    field_name: String,
    id: u8,
    serializer_id: u8,
}

#[derive(Deserialize)]
struct EntityTypeEntry {
    id: i32,
    name: String,
    client_tracking_range: i32,
    update_interval: i32,
}

pub(crate) fn build() -> (TokenStream, TokenStream, TokenStream) {
    println!("cargo:rerun-if-changed=build_assets/entities.json");

    let entities: Entities =
        serde_json::from_str(&fs::read_to_string("build_assets/entities.json").unwrap())
            .expect("Failed to parse entities.json");

    // Generate entity type constants
    let entity_type_consts: Vec<_> = entities
        .entity_types
        .iter()
        .map(|e| {
            let name = Ident::new(&e.name.to_uppercase(), Span::call_site());
            let id = e.id;
            // Tracking range is stored in chunks (multiply by 16 at runtime for blocks)
            let tracking_range_chunks = e.client_tracking_range;
            let update_interval = e.update_interval;
            let key = format!("minecraft:{}", e.name);
            quote! {
                pub const #name: EntityType = EntityType {
                    id: #id,
                    key: #key,
                    tracking_range_chunks: #tracking_range_chunks,
                    update_interval: #update_interval,
                };
            }
        })
        .collect();

    // Generate registry entries for ALL_ENTITY_TYPES array
    let entity_type_refs: Vec<_> = entities
        .entity_types
        .iter()
        .map(|e| {
            let name = Ident::new(&e.name.to_uppercase(), Span::call_site());
            quote! { &#name }
        })
        .collect();

    let entity_count = entities.entity_types.len();

    // Generate entity data serializer constants
    let serializer_consts: Vec<_> = entities
        .entity_data_serializers
        .iter()
        .map(|s| {
            let name = Ident::new(&s.name, Span::call_site());
            let id = s.id;
            quote! {
                pub const #name: u8 = #id;
            }
        })
        .collect();

    // Generate entity data accessor constants
    let accessor_consts: Vec<_> = entities
        .entity_data_accessors
        .entity
        .iter()
        .map(|a| {
            let name = Ident::new(&a.field_name, Span::call_site());
            let id = a.id;
            let serializer_id = a.serializer_id;
            quote! {
                pub const #name: EntityDataAccessor = EntityDataAccessor {
                    id: #id,
                    serializer_id: #serializer_id,
                };
            }
        })
        .collect();

    let entity_type_module = quote! {
        /// Entity type information (extracted from Minecraft)
        #[derive(Debug, Clone, Copy)]
        pub struct EntityType {
            /// Registry ID of the entity type
            pub id: i32,
            /// Registry key (e.g., "minecraft:player")
            pub key: &'static str,
            /// Client tracking range in chunks (multiply by 16 for blocks)
            pub tracking_range_chunks: i32,
            /// Update interval in ticks
            pub update_interval: i32,
        }

        impl EntityType {
            /// Returns the tracking range in blocks (tracking_range_chunks * 16)
            #[inline]
            #[must_use]
            pub const fn tracking_range_blocks(&self) -> i32 {
                self.tracking_range_chunks * 16
            }
        }

        #(#entity_type_consts)*

        /// All entity types in registry order
        pub static ALL_ENTITY_TYPES: [&EntityType; #entity_count] = [
            #(#entity_type_refs),*
        ];
    };

    let serializers_module = quote! {
        #(#serializer_consts)*
    };

    let accessors_module = quote! {
        /// Entity data accessor information
        #[derive(Debug, Clone, Copy)]
        pub struct EntityDataAccessor {
            /// Field ID
            pub id: u8,
            /// Serializer ID
            pub serializer_id: u8,
        }

        #(#accessor_consts)*
    };

    (entity_type_module, serializers_module, accessors_module)
}
