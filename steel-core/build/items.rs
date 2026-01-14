//! Code generation for item behaviors.

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ItemClass {
    pub name: String,
    pub class: String,
    #[serde(default)]
    pub block: Option<String>,
}

/// Items use lowercase field names (`vanilla_items::ITEMS.stone`)
fn to_item_field(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Blocks use `SCREAMING_SNAKE_CASE` constants (`vanilla_blocks::STONE`)
fn to_block_const(name: &str) -> Ident {
    Ident::new(&name.to_shouty_snake_case(), Span::call_site())
}

pub fn build(items: &[ItemClass]) -> String {
    let mut block_item_registrations = TokenStream::new();

    for item in items {
        if (item.class == "BlockItem" || item.class == "DoubleHighBlockItem")
            && let Some(block) = &item.block
        {
            let item_field = to_item_field(&item.name);
            let block_const = to_block_const(block);

            block_item_registrations.extend(quote! {
                registry.set_behavior(
                    &vanilla_items::ITEMS.#item_field,
                    Box::new(BlockItemBehavior::new(vanilla_blocks::#block_const)),
                );
            });
        }
    }

    let output = quote! {
        //! Generated item behavior assignments.

        use steel_registry::{vanilla_blocks, vanilla_items};
        use crate::behavior::ItemBehaviorRegistry;
        use crate::behavior::items::BlockItemBehavior;

        pub fn register_item_behaviors(registry: &mut ItemBehaviorRegistry) {
            #block_item_registrations
        }
    };

    output.to_string()
}
