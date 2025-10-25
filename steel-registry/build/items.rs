use std::{collections::BTreeMap, fs};

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Item {
    pub id: u16,
    pub name: String,
    pub components: BTreeMap<String, Value>,
    pub block_item: Option<String>,
    pub is_double: bool,
    pub is_scaffolding: bool,
    pub is_water_placable: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Items {
    pub items: Vec<Item>,
}

fn get_component_ident(name: &str) -> Option<Ident> {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    let shouty_name = name.to_shouty_snake_case();
    Some(Ident::new(&shouty_name, Span::call_site()))
}

fn generate_builder_calls(item: &Item) -> Vec<TokenStream> {
    let mut builder_calls = Vec::new();

    for (key, value) in &item.components {
        let component_ident = if let Some(ident) = get_component_ident(key) {
            ident
        } else {
            continue;
        };

        match key.as_str() {
            "minecraft:max_stack_size" => {
                let val = value.as_i64().unwrap() as i32;
                if val != 64 {
                    builder_calls.push(
                        quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                    );
                }
            }
            "minecraft:max_damage" => {
                let val = value.as_i64().unwrap() as i32;
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                );
            }
            "minecraft:repair_cost" => {
                let val = value.as_i64().unwrap() as i32;
                if val != 0 {
                    builder_calls.push(
                        quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                    );
                }
            }
            "minecraft:unbreakable" => {
                builder_calls
                    .push(quote! { .builder_set(vanilla_components::#component_ident, Some(())) });
            }
            "minecraft:enchantment_glint_override" => {
                let val = value.as_bool().unwrap();
                builder_calls.push(
                    quote! { .builder_set(vanilla_components::#component_ident, Some(#val)) },
                );
            }
            _ => {
                // TODO: Implement more
            }
        }
    }

    builder_calls
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/items.json");
    let item_assets: Items =
        serde_json::from_str(&fs::read_to_string("build_assets/items.json").unwrap()).unwrap();

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::{
            data_components::{vanilla_components, DataComponentMap},
            vanilla_blocks,
            items::items::{Item, ItemRegistry},
        };
        use steel_utils::ResourceLocation;
        use std::sync::LazyLock;
    });

    for item in &item_assets.items {
        let item_name_upper = item.name.to_shouty_snake_case();
        let item_ident = Ident::new(&item_name_upper, Span::call_site());
        let item_name_str = item.name.clone();

        if let Some(block_name) = &item.block_item {
            let block_ident = Ident::new(&block_name.to_shouty_snake_case(), Span::call_site());

            if block_name != &item.name {
                stream.extend(quote! {
                    pub static #item_ident: LazyLock<Item> = LazyLock::new(|| Item::from_block_custom_name(vanilla_blocks::#block_ident, #item_name_str));
                });
            } else {
                stream.extend(quote! {
                    pub static #item_ident: LazyLock<Item> = LazyLock::new(|| Item::from_block(vanilla_blocks::#block_ident));
                });
            }
        } else {
            let builder_calls = generate_builder_calls(item);

            stream.extend(quote! {
                pub static #item_ident: LazyLock<Item> = LazyLock::new(|| Item {
                    key: ResourceLocation::vanilla_static(#item_name_str),
                    components: DataComponentMap::common_item_components()
                        #(#builder_calls)*,
                });
            });
        }
    }

    let mut register_stream = TokenStream::new();
    for item in &item_assets.items {
        let item_name = Ident::new(&item.name.to_shouty_snake_case(), Span::call_site());
        register_stream.extend(quote! {
            registry.register(&#item_name);
        });
    }

    stream.extend(quote! {
        pub fn register_items(registry: &mut ItemRegistry) {
            #register_stream
        }
    });

    stream
}
