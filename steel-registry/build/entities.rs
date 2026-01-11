use std::fs;

use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_json::Value;

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/entities.json");

    let content = fs::read_to_string("build_assets/entities.json").unwrap();
    let entity_types: Vec<Value> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse entities.json: {}", e));

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::entity_types::{EntityType, EntityTypeRegistry};
        use crate::Pose;
    });

    // Generate EntityDimensions struct
    stream.extend(quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct EntityDimensions {
            pub width: f32,
            pub height: f32,
            pub eye_height: f32,
        }

        impl EntityDimensions {
            pub const fn new(width: f32, height: f32, eye_height: f32) -> Self {
                Self { width, height, eye_height }
            }
        }
    });

    // Generate entity type constants
    for entity_type in &entity_types {
        let name = entity_type["name"].as_str().unwrap();
        let entity_type_ident = Ident::new(&name.to_shouty_snake_case(), Span::call_site());
        let id = entity_type["id"].as_i64().unwrap() as i32;
        let client_tracking_range = entity_type["client_tracking_range"].as_i64().unwrap() as i32;
        let update_interval = entity_type["update_interval"].as_i64().unwrap() as i32;

        stream.extend(quote! {
            pub const #entity_type_ident: &EntityType = &EntityType {
                key: #name,
                id: #id,
                client_tracking_range: #client_tracking_range,
                update_interval: #update_interval,
            };
        });
    }

    // Generate register function
    let mut register_stream = TokenStream::new();
    for entity_type in &entity_types {
        let name = entity_type["name"].as_str().unwrap();
        let entity_type_ident = Ident::new(&name.to_shouty_snake_case(), Span::call_site());
        register_stream.extend(quote! {
            registry.register(#entity_type_ident);
        });
    }

    stream.extend(quote! {
        pub fn register_entity_types(registry: &mut EntityTypeRegistry) {
            #register_stream
        }
    });

    // Generate dimension lookup functions for entities with pose_dimensions
    for entity_type in &entity_types {
        let name = entity_type["name"].as_str().unwrap();
        let width = entity_type["width"].as_f64().unwrap_or(0.6) as f32;
        let height = entity_type["height"].as_f64().unwrap_or(1.8) as f32;
        let eye_height = entity_type["eye_height"].as_f64().unwrap_or(1.62) as f32;

        if let Some(pose_dims) = entity_type["pose_dimensions"].as_object() {
            let fn_name = Ident::new(&format!("{}_dimensions_for_pose", name), Span::call_site());

            let mut pose_arms = TokenStream::new();
            for (pose_name, dims) in pose_dims {
                let pose_variant = Ident::new(&pose_name.to_upper_camel_case(), Span::call_site());
                let pose_width = dims["width"].as_f64().map(|v| v as f32).unwrap_or(width);
                let pose_height = dims["height"].as_f64().map(|v| v as f32).unwrap_or(height);
                let pose_eye_height = dims["eye_height"]
                    .as_f64()
                    .map(|v| v as f32)
                    .unwrap_or(eye_height);

                pose_arms.extend(quote! {
                    Pose::#pose_variant => EntityDimensions::new(#pose_width, #pose_height, #pose_eye_height),
                });
            }

            stream.extend(quote! {
                #[must_use]
                pub const fn #fn_name(pose: Pose) -> EntityDimensions {
                    match pose {
                        #pose_arms
                        _ => EntityDimensions::new(#width, #height, #eye_height),
                    }
                }
            });
        }
    }

    stream
}
