//! Build-time code generator for noise parameters.

use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_json::Value;

/// Generates noise parameter constants from the JSON file.
pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/noise_parameters.json");

    let json_file = fs::read_to_string("build_assets/noise_parameters.json")
        .expect("Failed to read noise_parameters.json");

    let params: serde_json::Map<String, Value> =
        serde_json::from_str(&json_file).expect("Failed to parse noise_parameters.json");

    let mut stream = TokenStream::new();

    // Generate the struct definition
    stream.extend(quote! {
        /// Parameters for double perlin noise generation.
        pub struct DoublePerlinNoiseParameters {
            /// The first octave level for the noise.
            pub first_octave: i32,
            /// Amplitude multipliers for each octave.
            pub amplitudes: &'static [f64],
            /// The identifier for this noise type.
            id: &'static str,
        }

        impl DoublePerlinNoiseParameters {
            /// Creates a new noise parameter set.
            pub const fn new(first_octave: i32, amplitudes: &'static [f64], id: &'static str) -> Self {
                Self {
                    first_octave,
                    amplitudes,
                    id,
                }
            }

            /// Returns the identifier for this noise type.
            pub const fn id(&self) -> &'static str {
                self.id
            }
        }
    });

    // Collect and sort parameter names for deterministic output
    let mut params_vec: Vec<_> = params.iter().collect();
    params_vec.sort_by_key(|(k, _)| *k);

    // Generate constants
    let mut const_names = Vec::new();
    let mut match_arms = Vec::new();

    for (key, value) in &params_vec {
        let obj = value.as_object().expect("Expected object");
        let first_octave = obj
            .get("firstOctave")
            .expect("Missing firstOctave")
            .as_i64()
            .expect("firstOctave must be i64") as i32;
        let amplitudes = obj
            .get("amplitudes")
            .expect("Missing amplitudes")
            .as_array()
            .expect("amplitudes must be array");

        let const_name_str = key.to_shouty_snake_case();
        let const_name = Ident::new(&const_name_str, Span::call_site());

        // Generate the amplitudes array
        let amps: Vec<_> = amplitudes
            .iter()
            .map(|v| v.as_f64().expect("amplitude must be f64"))
            .collect();
        let amps_tokens: Vec<_> = amps.iter().map(|a| quote! { #a }).collect();

        // The minecraft: prefix for the id
        let id = format!("minecraft:{key}");

        stream.extend(quote! {
            /// Noise parameters for #key.
            pub const #const_name: DoublePerlinNoiseParameters =
                DoublePerlinNoiseParameters::new(#first_octave, &[#(#amps_tokens),*], #id);
        });

        const_names.push(const_name.clone());
        match_arms.push(((*key).clone(), const_name));
    }

    // Generate id_to_parameters function
    let match_arms_tokens: Vec<_> = match_arms
        .iter()
        .map(|(key, const_name)| {
            quote! {
                #key => &#const_name,
            }
        })
        .collect();

    stream.extend(quote! {
        impl DoublePerlinNoiseParameters {
            /// Looks up noise parameters by their identifier (without minecraft: prefix).
            pub fn id_to_parameters(id: &str) -> Option<&'static DoublePerlinNoiseParameters> {
                Some(match id {
                    #(#match_arms_tokens)*
                    _ => return None,
                })
            }
        }
    });

    stream
}
