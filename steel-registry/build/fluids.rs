use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct FluidJson {
    id: Value, // Can be number or string
    name: String,
    #[serde(rename = "type")]
    fluid_type: String,
    #[serde(default)]
    bucket: Option<String>,
    #[serde(default)]
    properties: Vec<FluidPropertyJson>,
    #[serde(default)]
    states: Vec<Value>, // Keep as Value for flexibility
    #[serde(default)]
    #[serde(rename = "defaultStateIndex")]
    default_state_index: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct FluidPropertyJson {
    name: String,
    values: Vec<Value>, // Can be strings or numbers
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/fluids.json");

    // Try to read fluids.json, if not found use empty vec
    let fluids: Vec<FluidJson> = match fs::read_to_string("build_assets/fluids.json") {
        Ok(json) => serde_json::from_str(&json).expect("Failed to parse fluids.json"),
        Err(_) => {
            println!("cargo:warning=fluids.json not found - using empty fluid registry. Run SteelExtractor to generate fluid data.");
            Vec::new()
        }
    };

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::fluid::{FluidEntry, FluidId, FluidRegistry};
    });

    // Note: FluidId constants are defined in steel-registry/src/fluid/mod.rs
    // and should match the IDs in fluids.json:
    //   Empty = 0, Flowing_Water = 1, Water = 2, Flowing_Lava = 3, Lava = 4

    // Generate registration function
    let mut register_stream = TokenStream::new();
    for fluid in &fluids {
        let fluid_ident = Ident::new(&fluid.name.to_shouty_snake_case(), Span::call_site());
        let fluid_name_str = &fluid.name;

        // Extract id as u16 from either number or string
        let fluid_id: u16 = match &fluid.id {
            Value::Number(n) => n.as_u64().unwrap_or(0) as u16,
            Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };

        register_stream.extend(quote! {
            registry.register(FluidEntry {
                id: FluidId(#fluid_id),
                name: #fluid_name_str,
            });
        });
    }

    stream.extend(quote! {
        pub fn register_fluids(registry: &mut FluidRegistry) {
            #register_stream
        }
    });

    stream
}
