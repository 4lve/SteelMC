//! Build-time code generator for density functions.
//!
//! This module reads Minecraft's vanilla `noise_settings` JSON files and generates
//! Rust code for the noise router density functions. It parses the vanilla JSON
//! format directly using Serde.

use std::fs;

use crate::density_function_type::{DensityFunction, DensityFunctionNode, FlattenContext};
use crate::density_function_utils::DATAPACK_BASE;
use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;

// =============================================================================
// Generated Handlers
// =============================================================================

fn emit_stack(ctx: &FlattenContext, stack_name: &Ident) -> Option<TokenStream> {
    let components = &ctx.stack;
    let static_data = &ctx.static_data;
    let splines = &ctx.splines;
    let components_len = components.len();

    if components_len == 0 {
        return None;
    }

    Some(quote! {
        #(#static_data)*
        #(#splines)*
        static #stack_name: [BaseNoiseFunctionComponent; #components_len] = [
            #(#components),*
        ];
    })
}

struct NoiseStackResult {
    ctx: FlattenContext,
    indices: FxHashMap<&'static str, usize>,
    stack_name: Ident,
}

const NOISE_FIELDS: [(&str, &str); 10] = [
    ("barrier", "barrierNoise"),
    ("fluid_level_floodedness", "fluidLevelFloodednessNoise"),
    ("fluid_level_spread", "fluidLevelSpreadNoise"),
    ("lava", "lavaNoise"),
    ("erosion", "erosion"),
    ("depth", "depth"),
    ("final_density", "finalDensity"),
    ("vein_toggle", "veinToggle"),
    ("vein_ridged", "veinRidged"),
    ("vein_gap", "veinGap"),
];

fn build_noise_stack(
    env_name: &str,
    env_name_upper: &str,
    env_data: &FxHashMap<String, DensityFunctionNode>,
) -> NoiseStackResult {
    let mut noise_ctx = FlattenContext::new(env_name);
    let mut noise_indices: FxHashMap<&str, usize> = FxHashMap::default();

    for (json_name, internal_name) in NOISE_FIELDS {
        if let Some(node) = env_data.get(json_name) {
            let idx = node.flatten(&mut noise_ctx);
            noise_indices.insert(internal_name, idx);
        }
    }

    if let Some(&original_final) = noise_indices.get("finalDensity") {
        let beardifier_idx = noise_ctx.stack.len();
        noise_ctx
            .stack
            .push(quote! { BaseNoiseFunctionComponent::Beardifier });

        let add_data_name = noise_ctx.next_data_name("BINARY_DATA");
        noise_ctx.static_data.push(quote! {
            static #add_data_name: BinaryData = BinaryData { operation: BinaryOperation::Add };
        });

        let add_idx = noise_ctx.stack.len();
        noise_ctx.stack.push(quote! {
            BaseNoiseFunctionComponent::Binary {
                argument1_index: #original_final,
                argument2_index: #beardifier_idx,
                data: &#add_data_name,
            }
        });

        let cell_cache_idx = noise_ctx.stack.len();
        noise_ctx.stack.push(quote! {
            BaseNoiseFunctionComponent::Wrapper {
                input_index: #add_idx,
                wrapper: WrapperType::CellCache,
            }
        });

        noise_indices.insert("finalDensity", cell_cache_idx);
    }

    let stack_name = Ident::new(&format!("{env_name_upper}_NOISE_STACK"), Span::call_site());

    NoiseStackResult {
        ctx: noise_ctx,
        indices: noise_indices,
        stack_name,
    }
}

fn build_surface_stack(
    env_name: &str,
    env_name_upper: &str,
    noise_router: &FxHashMap<String, DensityFunctionNode>,
) -> (FlattenContext, Ident) {
    let surface_prefix = format!("{env_name}_surface");
    let mut ctx = FlattenContext::new(&surface_prefix);

    if let Some(node) = noise_router.get("preliminary_surface_level") {
        match node {
            DensityFunctionNode::Function(func) => {
                if let DensityFunction::FindTopSurface(inner) = &**func {
                    if let Some(d) = &inner.density {
                        d.flatten(&mut ctx);
                    }
                } else {
                    node.flatten(&mut ctx);
                }
            }
            _ => {
                node.flatten(&mut ctx);
            }
        }
    }

    if ctx.stack.is_empty() {
        ctx.stack
            .push(quote! { BaseNoiseFunctionComponent::Constant { value: 0f64 } });
    }

    let stack_name = Ident::new(
        &format!("{env_name_upper}_SURFACE_STACK"),
        Span::call_site(),
    );
    (ctx, stack_name)
}

struct MultiNoiseStackResult {
    ctx: FlattenContext,
    indices: FxHashMap<&'static str, usize>,
    stack_name: Ident,
}

const MULTI_FIELDS: [&str; 6] = [
    "temperature",
    "vegetation",
    "continents",
    "erosion",
    "depth",
    "ridges",
];

fn build_multi_noise_stack(
    env_name: &str,
    env_name_upper: &str,
    noise_router: &FxHashMap<String, DensityFunctionNode>,
) -> MultiNoiseStackResult {
    let multi_prefix = format!("{env_name}_multi");
    let mut multi_ctx = FlattenContext::new(&multi_prefix);

    let mut multi_indices: FxHashMap<&'static str, usize> = FxHashMap::default();
    for json_name in MULTI_FIELDS {
        if let Some(node) = noise_router.get(json_name) {
            let idx = node.flatten(&mut multi_ctx);
            multi_indices.insert(json_name, idx);
        }
    }

    let stack_name = Ident::new(&format!("{env_name_upper}_MULTI_STACK"), Span::call_site());

    MultiNoiseStackResult {
        ctx: multi_ctx,
        indices: multi_indices,
        stack_name,
    }
}

fn build_surface_estimator_tokens(
    surface_stream: Option<&TokenStream>,
    surface_stack_name: &Ident,
) -> TokenStream {
    if surface_stream.is_some() {
        quote! {
            surface_estimator: BaseSurfaceEstimator {
                full_component_stack: &#surface_stack_name,
            },
        }
    } else {
        quote! {
            surface_estimator: BaseSurfaceEstimator {
                full_component_stack: &[],
            },
        }
    }
}

fn build_multi_noise_tokens(
    multi_stream: Option<&TokenStream>,
    multi_stack_name: &Ident,
    multi_indices: &FxHashMap<&'static str, usize>,
) -> TokenStream {
    if multi_stream.is_some() {
        let temperature = multi_indices.get("temperature").copied().unwrap_or(0);
        let vegetation = multi_indices.get("vegetation").copied().unwrap_or(0);
        let continents = multi_indices.get("continents").copied().unwrap_or(0);
        let erosion = multi_indices.get("erosion").copied().unwrap_or(0);
        let depth = multi_indices.get("depth").copied().unwrap_or(0);
        let ridges = multi_indices.get("ridges").copied().unwrap_or(0);

        quote! {
            multi_noise: BaseMultiNoiseRouter {
                full_component_stack: &#multi_stack_name,
                temperature: #temperature,
                vegetation: #vegetation,
                continents: #continents,
                erosion: #erosion,
                depth: #depth,
                ridges: #ridges,
            },
        }
    } else {
        quote! {
            multi_noise: BaseMultiNoiseRouter {
                full_component_stack: &[],
                temperature: 0,
                vegetation: 0,
                continents: 0,
                erosion: 0,
                depth: 0,
                ridges: 0,
            },
        }
    }
}

fn build_noise_router_tokens(
    noise_stream: Option<&TokenStream>,
    noise_stack_name: &Ident,
    noise_indices: &FxHashMap<&'static str, usize>,
) -> TokenStream {
    if noise_stream.is_some() {
        let barrier_noise = noise_indices.get("barrierNoise").copied().unwrap_or(0);
        let fluid_level_floodedness = noise_indices
            .get("fluidLevelFloodednessNoise")
            .copied()
            .unwrap_or(0);
        let fluid_level_spread = noise_indices
            .get("fluidLevelSpreadNoise")
            .copied()
            .unwrap_or(0);
        let lava_noise = noise_indices.get("lavaNoise").copied().unwrap_or(0);
        let erosion = noise_indices.get("erosion").copied().unwrap_or(0);
        let depth = noise_indices.get("depth").copied().unwrap_or(0);
        let final_density = noise_indices.get("finalDensity").copied().unwrap_or(0);
        let vein_toggle = noise_indices.get("veinToggle").copied().unwrap_or(0);
        let vein_ridged = noise_indices.get("veinRidged").copied().unwrap_or(0);
        let vein_gap = noise_indices.get("veinGap").copied().unwrap_or(0);

        quote! {
            noise: BaseNoiseRouter {
                full_component_stack: &#noise_stack_name,
                barrier_noise: #barrier_noise,
                fluid_level_floodedness_noise: #fluid_level_floodedness,
                fluid_level_spread_noise: #fluid_level_spread,
                lava_noise: #lava_noise,
                erosion: #erosion,
                depth: #depth,
                final_density: #final_density,
                vein_toggle: #vein_toggle,
                vein_ridged: #vein_ridged,
                vein_gap: #vein_gap,
            },
        }
    } else {
        quote! {
            noise: BaseNoiseRouter {
                full_component_stack: &[],
                barrier_noise: 0,
                fluid_level_floodedness_noise: 0,
                fluid_level_spread_noise: 0,
                lava_noise: 0,
                erosion: 0,
                depth: 0,
                final_density: 0,
                vein_toggle: 0,
                vein_ridged: 0,
                vein_gap: 0,
            },
        }
    }
}

fn generate_environment(
    env_name: &str,
    noise_router: &FxHashMap<String, DensityFunctionNode>,
) -> (TokenStream, String) {
    let env_name_upper = env_name.to_shouty_snake_case();

    let noise_result = build_noise_stack(env_name, &env_name_upper, noise_router);
    let (surface_ctx, surface_stack_name) =
        build_surface_stack(env_name, &env_name_upper, noise_router);
    let multi_result = build_multi_noise_stack(env_name, &env_name_upper, noise_router);

    let noise_stream = emit_stack(&noise_result.ctx, &noise_result.stack_name);
    let surface_stream = emit_stack(&surface_ctx, &surface_stack_name);
    let multi_stream = emit_stack(&multi_result.ctx, &multi_result.stack_name);

    let router_name = Ident::new(
        &format!("{env_name_upper}_BASE_NOISE_ROUTER"),
        Span::call_site(),
    );

    let noise_router_tokens = build_noise_router_tokens(
        noise_stream.as_ref(),
        &noise_result.stack_name,
        &noise_result.indices,
    );

    let surface_estimator_tokens =
        build_surface_estimator_tokens(surface_stream.as_ref(), &surface_stack_name);

    let multi_noise_tokens = build_multi_noise_tokens(
        multi_stream.as_ref(),
        &multi_result.stack_name,
        &multi_result.indices,
    );

    let mut stream = TokenStream::new();
    if let Some(ns) = noise_stream {
        stream.extend(ns);
    }
    if let Some(ss) = surface_stream {
        stream.extend(ss);
    }
    if let Some(ms) = multi_stream {
        stream.extend(ms);
    }

    stream.extend(quote! {
        pub static #router_name: BaseNoiseRouters = BaseNoiseRouters {
            #noise_router_tokens
            #surface_estimator_tokens
            #multi_noise_tokens
        };
    });

    (stream, router_name.to_string())
}

pub(crate) fn build() -> TokenStream {
    let noise_settings_path = format!("{DATAPACK_BASE}/worldgen/noise_settings");
    let density_function_path = format!("{DATAPACK_BASE}/worldgen/density_function");

    println!("cargo:rerun-if-changed={noise_settings_path}");
    println!("cargo:rerun-if-changed={density_function_path}");

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::noise_router::types::{
            BaseNoiseFunctionComponent, BaseNoiseRouter, BaseNoiseRouters, BaseSurfaceEstimator,
            BaseMultiNoiseRouter, NoiseData, ShiftedNoiseData, ClampedYGradientData,
            BinaryData, BinaryOperation, UnaryData, UnaryOperation,
            ClampData, RangeChoiceData, WeirdScaledData, WeirdScaledMapper,
            InterpolatedNoiseSamplerData, WrapperType, SplineRepr, SplinePoint,
        };
    });

    let environments = [
        ("overworld", "overworld.json"),
        ("amplified", "amplified.json"),
        ("large_biomes", "large_biomes.json"),
        ("nether", "nether.json"),
        ("end", "end.json"),
        ("caves", "caves.json"),
        ("floating_islands", "floating_islands.json"),
    ];

    for (env_name, file_name) in environments {
        let file_path = format!("{DATAPACK_BASE}/worldgen/noise_settings/{file_name}");

        let Ok(json_content) = fs::read_to_string(&file_path) else {
            eprintln!("Note: Skipping {env_name} - file not found at {file_path}");
            continue;
        };

        let noise_settings: serde_json::Value = match serde_json::from_str(&json_content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: Failed to parse {file_path}: {e}");
                continue;
            }
        };

        let Some(noise_router_value) = noise_settings.get("noise_router") else {
            eprintln!("Warning: No noise_router found in {file_path}");
            continue;
        };

        let noise_router: FxHashMap<String, DensityFunctionNode> =
            match serde_json::from_value(noise_router_value.clone()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Warning: Failed to parse noise_router in {file_path}: {e}");
                    continue;
                }
            };

        let (env_stream, _router_name) = generate_environment(env_name, &noise_router);
        stream.extend(env_stream);
    }

    stream
}
