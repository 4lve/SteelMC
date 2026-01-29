//! Build-time code generator for density functions.
//!
//! This module reads Minecraft's vanilla `noise_settings` JSON files and generates
//! Rust code for the noise router density functions. It parses the vanilla JSON
//! format directly using Serde.

use std::fs;

use enum_dispatch::enum_dispatch;
use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Base path for builtin datapacks
const DATAPACK_BASE: &str =
    "../steel-registry/build_assets/builtin_datapacks/minecraft/data/minecraft";

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum DensityFunctionNode {
    Constant(f64),
    Reference(String),
    Function(Box<DensityFunction>),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum DensityFunction {
    #[serde(rename = "minecraft:blend_offset")]
    BlendOffset,
    #[serde(rename = "minecraft:blend_alpha")]
    BlendAlpha,
    #[serde(rename = "minecraft:end_islands")]
    EndIslands,
    #[serde(rename = "minecraft:beardifier")]
    Beardifier,
    #[serde(rename = "minecraft:old_blended_noise")]
    OldBlendedNoise,
    #[serde(rename = "minecraft:y_clamped_gradient")]
    YClampedGradient {
        from_y: i32,
        to_y: i32,
        from_value: f64,
        to_value: f64,
    },
    #[serde(rename = "minecraft:noise")]
    Noise {
        noise: String,
        #[serde(default = "default_scale")]
        xz_scale: f64,
        #[serde(default = "default_scale")]
        y_scale: f64,
    },
    #[serde(rename = "minecraft:shift_a")]
    ShiftA { argument: String },
    #[serde(rename = "minecraft:shift_b")]
    ShiftB { argument: String },
    #[serde(rename = "minecraft:shifted_noise")]
    ShiftedNoise {
        noise: String,
        #[serde(default = "default_scale")]
        xz_scale: f64,
        #[serde(default = "default_scale")]
        y_scale: f64,
        shift_x: DensityFunctionNode,
        shift_y: DensityFunctionNode,
        shift_z: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:interpolated")]
    Interpolated { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:flat_cache")]
    FlatCache { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:cache_2d")]
    Cache2D { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:cache_once")]
    CacheOnce { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:cache_all_in_cell")]
    CacheAllInCell { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:add")]
    Add {
        argument1: DensityFunctionNode,
        argument2: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:mul")]
    Mul {
        argument1: DensityFunctionNode,
        argument2: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:min")]
    Min {
        argument1: DensityFunctionNode,
        argument2: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:max")]
    Max {
        argument1: DensityFunctionNode,
        argument2: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:abs")]
    Abs { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:square")]
    Square { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:cube")]
    Cube { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:half_negative")]
    HalfNegative { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:quarter_negative")]
    QuarterNegative { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:squeeze")]
    Squeeze { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:clamp")]
    Clamp {
        input: DensityFunctionNode,
        min: f64,
        max: f64,
    },
    #[serde(rename = "minecraft:range_choice")]
    RangeChoice {
        input: DensityFunctionNode,
        min_inclusive: f64,
        max_exclusive: f64,
        when_in_range: DensityFunctionNode,
        when_out_of_range: DensityFunctionNode,
    },
    #[serde(rename = "minecraft:blend_density")]
    BlendDensity { argument: DensityFunctionNode },
    #[serde(rename = "minecraft:weird_scaled_sampler")]
    WeirdScaledSampler {
        input: DensityFunctionNode,
        rarity_value_mapper: String,
        noise: String,
    },
    #[serde(rename = "minecraft:spline")]
    Spline { spline: SplineData },
    #[serde(rename = "minecraft:find_top_surface")]
    FindTopSurface {
        density: Option<DensityFunctionNode>,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum SplineData {
    Fixed(f64),
    Spline {
        coordinate: DensityFunctionNode,
        points: Vec<SplinePoint>,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SplinePoint {
    location: f64,
    derivative: f64,
    value: SplineData,
}

fn default_scale() -> f64 {
    1.0
}

/// Context for flattening density functions with reference resolution
struct FlattenContext {
    env_prefix: String,
    stack: Vec<TokenStream>,
    seen: FxHashMap<String, usize>,
    static_data: Vec<TokenStream>,
    data_counter: usize,
    splines: Vec<TokenStream>,
    spline_counter: usize,
    /// Cache for resolved density function references
    ref_cache: FxHashMap<String, DensityFunctionNode>,
}

impl FlattenContext {
    fn new(env_name: &str) -> Self {
        Self {
            env_prefix: env_name.to_shouty_snake_case(),
            stack: Vec::new(),
            seen: FxHashMap::default(),
            static_data: Vec::new(),
            data_counter: 0,
            splines: Vec::new(),
            spline_counter: 0,
            ref_cache: FxHashMap::default(),
        }
    }

    fn next_data_name(&mut self, prefix: &str) -> Ident {
        let name = format!("{}_{prefix}_{}", self.env_prefix, self.data_counter);
        self.data_counter += 1;
        Ident::new(&name, Span::call_site())
    }

    fn next_spline_name(&mut self) -> Ident {
        let name = format!("{}_SPLINE_{}", self.env_prefix, self.spline_counter);
        self.spline_counter += 1;
        Ident::new(&name, Span::call_site())
    }

    /// Resolve a density function reference like "minecraft:overworld/continents"
    fn resolve_reference(&mut self, reference: &str) -> DensityFunctionNode {
        let path = reference.strip_prefix("minecraft:").unwrap_or(reference);

        if let Some(cached) = self.ref_cache.get(path) {
            return cached.clone();
        }

        let file_path = format!("{DATAPACK_BASE}/worldgen/density_function/{path}.json");

        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to read density function at {file_path}: {e}"));

        let value: DensityFunctionNode = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse density function at {file_path}: {e}"));

        self.ref_cache.insert(path.to_string(), value.clone());
        value
    }
}

fn hash_node(value: &DensityFunctionNode) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn strip_minecraft_prefix(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

// =============================================================================
// Generated Handlers
// =============================================================================

fn handle_old_blended_noise(ctx: &mut FlattenContext) -> TokenStream {
    let data_name = ctx.next_data_name("BLENDED_NOISE_DATA");
    ctx.static_data.push(quote! {
        static #data_name: InterpolatedNoiseSamplerData = InterpolatedNoiseSamplerData {
            scaled_xz_scale: 171.103_f64,
            scaled_y_scale: 85.551_5_f64,
            xz_factor: 80_f64,
            y_factor: 160_f64,
            smear_scale_multiplier: 8_f64,
        };
    });
    quote! { BaseNoiseFunctionComponent::InterpolatedNoiseSampler { data: &#data_name } }
}

fn handle_constant_value(val: f64) -> TokenStream {
    quote! { BaseNoiseFunctionComponent::Constant { value: #val } }
}

fn handle_y_clamped_gradient(
    ctx: &mut FlattenContext,
    from_y: i32,
    to_y: i32,
    from_value: f64,
    to_value: f64,
) -> TokenStream {
    let from_y = f64::from(from_y);
    let to_y = f64::from(to_y);
    let data_name = ctx.next_data_name("Y_GRADIENT_DATA");
    ctx.static_data.push(quote! {
        static #data_name: ClampedYGradientData = ClampedYGradientData {
            from_y: #from_y,
            to_y: #to_y,
            from_value: #from_value,
            to_value: #to_value,
        };
    });
    quote! { BaseNoiseFunctionComponent::ClampedYGradient { data: &#data_name } }
}

fn handle_noise(
    ctx: &mut FlattenContext,
    noise_id: &str,
    xz_scale: f64,
    y_scale: f64,
) -> TokenStream {
    let noise_id = strip_minecraft_prefix(noise_id);
    let data_name = ctx.next_data_name("NOISE_DATA");
    ctx.static_data.push(quote! {
        static #data_name: NoiseData = NoiseData {
            noise_id: #noise_id,
            xz_scale: #xz_scale,
            y_scale: #y_scale,
        };
    });
    quote! { BaseNoiseFunctionComponent::Noise { data: &#data_name } }
}

fn handle_shifted_noise(
    ctx: &mut FlattenContext,
    noise_id: &str,
    xz_scale: f64,
    y_scale: f64,
    shift_x: &DensityFunctionNode,
    shift_y: &DensityFunctionNode,
    shift_z: &DensityFunctionNode,
) -> TokenStream {
    let noise_id = strip_minecraft_prefix(noise_id);

    let idx_for_x = flatten_node(ctx, shift_x);
    let idx_for_y = flatten_node(ctx, shift_y);
    let idx_for_z = flatten_node(ctx, shift_z);

    let data_name = ctx.next_data_name("SHIFTED_NOISE_DATA");
    ctx.static_data.push(quote! {
        static #data_name: ShiftedNoiseData = ShiftedNoiseData {
            xz_scale: #xz_scale,
            y_scale: #y_scale,
            noise_id: #noise_id,
        };
    });

    quote! {
        BaseNoiseFunctionComponent::ShiftedNoise {
            shift_x_index: #idx_for_x,
            shift_y_index: #idx_for_y,
            shift_z_index: #idx_for_z,
            data: &#data_name,
        }
    }
}

fn handle_cache(
    ctx: &mut FlattenContext,
    wrapped_node: &DensityFunctionNode,
    variant: TokenStream,
) -> TokenStream {
    let input_idx = flatten_node(ctx, wrapped_node);

    quote! {
        BaseNoiseFunctionComponent::Wrapper {
            input_index: #input_idx,
            wrapper: #variant,
        }
    }
}

fn handle_binary_operation(
    ctx: &mut FlattenContext,
    arg1: &DensityFunctionNode,
    arg2: &DensityFunctionNode,
    op: TokenStream,
) -> TokenStream {
    let idx_arg1 = flatten_node(ctx, arg1);
    let idx_arg2 = flatten_node(ctx, arg2);

    let data_name = ctx.next_data_name("BINARY_DATA");
    ctx.static_data.push(quote! {
        static #data_name: BinaryData = BinaryData { operation: #op };
    });

    quote! {
        BaseNoiseFunctionComponent::Binary {
            argument1_index: #idx_arg1,
            argument2_index: #idx_arg2,
            data: &#data_name,
        }
    }
}

fn handle_unary_operation(
    ctx: &mut FlattenContext,
    input: &DensityFunctionNode,
    op: TokenStream,
) -> TokenStream {
    let input_idx = flatten_node(ctx, input);

    let data_name = ctx.next_data_name("UNARY_DATA");
    ctx.static_data.push(quote! {
        static #data_name: UnaryData = UnaryData { operation: #op };
    });

    quote! {
        BaseNoiseFunctionComponent::Unary {
            input_index: #input_idx,
            data: &#data_name,
        }
    }
}

fn handle_clamp(
    ctx: &mut FlattenContext,
    input: &DensityFunctionNode,
    min_val: f64,
    max_val: f64,
) -> TokenStream {
    let input_idx = flatten_node(ctx, input);

    let data_name = ctx.next_data_name("CLAMP_DATA");
    ctx.static_data.push(quote! {
        static #data_name: ClampData = ClampData {
            min_value: #min_val,
            max_value: #max_val,
        };
    });

    quote! {
        BaseNoiseFunctionComponent::Clamp {
            input_index: #input_idx,
            data: &#data_name,
        }
    }
}

fn handle_range_choice(
    ctx: &mut FlattenContext,
    input: &DensityFunctionNode,
    min_inclusive: f64,
    max_exclusive: f64,
    when_in_range: &DensityFunctionNode,
    when_out_of_range: &DensityFunctionNode,
) -> TokenStream {
    let input_idx = flatten_node(ctx, input);
    let when_in_idx = flatten_node(ctx, when_in_range);
    let when_out_idx = flatten_node(ctx, when_out_of_range);

    let data_name = ctx.next_data_name("RANGE_CHOICE_DATA");
    ctx.static_data.push(quote! {
        static #data_name: RangeChoiceData = RangeChoiceData {
            min_inclusive: #min_inclusive,
            max_exclusive: #max_exclusive,
        };
    });

    quote! {
        BaseNoiseFunctionComponent::RangeChoice {
            input_index: #input_idx,
            when_in_range_index: #when_in_idx,
            when_out_range_index: #when_out_idx,
            data: &#data_name,
        }
    }
}

fn handle_weird_scaled_sampler(
    ctx: &mut FlattenContext,
    input: &DensityFunctionNode,
    noise_id: &str,
    rarity_mapper: &str,
) -> TokenStream {
    let noise_id = strip_minecraft_prefix(noise_id);
    let input_idx = flatten_node(ctx, input);

    let mapper = match rarity_mapper {
        "type_1" => quote! { WeirdScaledMapper::Tunnels },
        "type_2" => quote! { WeirdScaledMapper::Caves },
        _ => panic!("Unknown rarity mapper: {rarity_mapper}"),
    };

    let data_name = ctx.next_data_name("WEIRD_SCALED_DATA");
    ctx.static_data.push(quote! {
        static #data_name: WeirdScaledData = WeirdScaledData {
            noise_id: #noise_id,
            mapper: #mapper,
        };
    });

    quote! {
        BaseNoiseFunctionComponent::WeirdScaled {
            input_index: #input_idx,
            data: &#data_name,
        }
    }
}

fn handle_spline_component(ctx: &mut FlattenContext, spline: &SplineData) -> TokenStream {
    let spline_name = flatten_spline(ctx, spline);
    quote! { BaseNoiseFunctionComponent::Spline { spline: &#spline_name } }
}

/// Main node flattening function - parses vanilla JSON directly
fn flatten_node(ctx: &mut FlattenContext, node: &DensityFunctionNode) -> usize {
    match node {
        DensityFunctionNode::Constant(val) => {
            let hash = format!("const:{val}");
            if let Some(&idx) = ctx.seen.get(&hash) {
                return idx;
            }
            let component = handle_constant_value(*val);
            let idx = ctx.stack.len();
            ctx.stack.push(component);
            ctx.seen.insert(hash, idx);
            idx
        }
        DensityFunctionNode::Reference(ref_str) => {
            let resolved = ctx.resolve_reference(ref_str);
            flatten_node(ctx, &resolved)
        }
        DensityFunctionNode::Function(func) => {
            let hash = hash_node(node);
            if let Some(&idx) = ctx.seen.get(&hash) {
                return idx;
            }

            let component = match &**func {
                DensityFunction::BlendOffset => quote! { BaseNoiseFunctionComponent::BlendOffset },
                DensityFunction::BlendAlpha => quote! { BaseNoiseFunctionComponent::BlendAlpha },
                DensityFunction::EndIslands => quote! { BaseNoiseFunctionComponent::EndIslands },
                DensityFunction::Beardifier => quote! { BaseNoiseFunctionComponent::Beardifier },
                DensityFunction::OldBlendedNoise => handle_old_blended_noise(ctx),
                DensityFunction::YClampedGradient {
                    from_y,
                    to_y,
                    from_value,
                    to_value,
                } => handle_y_clamped_gradient(ctx, *from_y, *to_y, *from_value, *to_value),
                DensityFunction::Noise {
                    noise,
                    xz_scale,
                    y_scale,
                } => handle_noise(ctx, noise, *xz_scale, *y_scale),
                DensityFunction::ShiftA { argument } => {
                    let noise_id = strip_minecraft_prefix(argument);
                    quote! { BaseNoiseFunctionComponent::ShiftA { noise_id: #noise_id } }
                }
                DensityFunction::ShiftB { argument } => {
                    let noise_id = strip_minecraft_prefix(argument);
                    quote! { BaseNoiseFunctionComponent::ShiftB { noise_id: #noise_id } }
                }
                DensityFunction::ShiftedNoise {
                    noise,
                    xz_scale,
                    y_scale,
                    shift_x,
                    shift_y,
                    shift_z,
                } => handle_shifted_noise(
                    ctx, noise, *xz_scale, *y_scale, shift_x, shift_y, shift_z,
                ),
                DensityFunction::Interpolated { argument } => {
                    handle_cache(ctx, argument, quote! { WrapperType::Interpolated })
                }
                DensityFunction::FlatCache { argument } => {
                    handle_cache(ctx, argument, quote! { WrapperType::CacheFlat })
                }
                DensityFunction::Cache2D { argument } => {
                    handle_cache(ctx, argument, quote! { WrapperType::Cache2D })
                }
                DensityFunction::CacheOnce { argument } => {
                    handle_cache(ctx, argument, quote! { WrapperType::CacheOnce })
                }
                DensityFunction::CacheAllInCell { argument } => {
                    handle_cache(ctx, argument, quote! { WrapperType::CellCache })
                }
                DensityFunction::Add {
                    argument1,
                    argument2,
                } => handle_binary_operation(
                    ctx,
                    argument1,
                    argument2,
                    quote! { BinaryOperation::Add },
                ),
                DensityFunction::Mul {
                    argument1,
                    argument2,
                } => handle_binary_operation(
                    ctx,
                    argument1,
                    argument2,
                    quote! { BinaryOperation::Mul },
                ),
                DensityFunction::Min {
                    argument1,
                    argument2,
                } => handle_binary_operation(
                    ctx,
                    argument1,
                    argument2,
                    quote! { BinaryOperation::Min },
                ),
                DensityFunction::Max {
                    argument1,
                    argument2,
                } => handle_binary_operation(
                    ctx,
                    argument1,
                    argument2,
                    quote! { BinaryOperation::Max },
                ),
                DensityFunction::Abs { argument } => {
                    handle_unary_operation(ctx, argument, quote! { UnaryOperation::Abs })
                }
                DensityFunction::Square { argument } => {
                    handle_unary_operation(ctx, argument, quote! { UnaryOperation::Square })
                }
                DensityFunction::Cube { argument } => {
                    handle_unary_operation(ctx, argument, quote! { UnaryOperation::Cube })
                }
                DensityFunction::HalfNegative { argument } => {
                    handle_unary_operation(ctx, argument, quote! { UnaryOperation::HalfNegative })
                }
                DensityFunction::QuarterNegative { argument } => {
                    handle_unary_operation(
                        ctx,
                        argument,
                        quote! { UnaryOperation::QuarterNegative },
                    )
                }
                DensityFunction::Squeeze { argument } => {
                    handle_unary_operation(ctx, argument, quote! { UnaryOperation::Squeeze })
                }
                DensityFunction::Clamp { input, min, max } => handle_clamp(ctx, input, *min, *max),
                DensityFunction::RangeChoice {
                    input,
                    min_inclusive,
                    max_exclusive,
                    when_in_range,
                    when_out_of_range,
                } => handle_range_choice(
                    ctx,
                    input,
                    *min_inclusive,
                    *max_exclusive,
                    when_in_range,
                    when_out_of_range,
                ),
                DensityFunction::BlendDensity { argument } => {
                    let input_idx = flatten_node(ctx, argument);
                    quote! { BaseNoiseFunctionComponent::BlendDensity { input_index: #input_idx } }
                }
                DensityFunction::WeirdScaledSampler {
                    input,
                    rarity_value_mapper,
                    noise,
                } => handle_weird_scaled_sampler(ctx, input, noise, rarity_value_mapper),
                DensityFunction::Spline { spline } => handle_spline_component(ctx, spline),
                DensityFunction::FindTopSurface { density } => {
                    if let Some(d) = density {
                        flatten_node(ctx, d);
                    }
                    quote! { BaseNoiseFunctionComponent::Constant { value: 0.0 } }
                }
            };

            let idx = ctx.stack.len();
            ctx.stack.push(component);
            ctx.seen.insert(hash, idx);
            idx
        }
    }
}

fn flatten_spline(ctx: &mut FlattenContext, spline: &SplineData) -> Ident {
    let spline_name = ctx.next_spline_name();

    match spline {
        SplineData::Fixed(val) => {
            let value_f32 = *val as f32;
            ctx.splines.push(quote! {
                static #spline_name: SplineRepr = SplineRepr::Fixed { value: #value_f32 };
            });
        }
        SplineData::Spline { coordinate, points } => {
            let coord_idx = flatten_node(ctx, coordinate);

            let mut point_tokens = Vec::new();

            for point in points {
                let location = point.location as f32;
                let derivative = point.derivative as f32;
                let value_spline_name = flatten_spline(ctx, &point.value);

                point_tokens.push(quote! {
                    SplinePoint {
                        location: #location,
                        value: &#value_spline_name,
                        derivative: #derivative,
                    }
                });
            }

            let points_len = point_tokens.len();
            let points_name = Ident::new(&format!("{spline_name}_POINTS"), Span::call_site());
            ctx.splines.push(quote! {
                static #points_name: [SplinePoint; #points_len] = [#(#point_tokens),*];
            });

            ctx.splines.push(quote! {
                static #spline_name: SplineRepr = SplineRepr::Standard {
                    location_function_index: #coord_idx,
                    points: &#points_name,
                };
            });
        }
    }

    spline_name
}

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
            let idx = flatten_node(&mut noise_ctx, node);
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
                if let DensityFunction::FindTopSurface { density } = &**func {
                    if let Some(d) = density {
                        flatten_node(&mut ctx, d);
                    }
                } else {
                    flatten_node(&mut ctx, node);
                }
            }
            _ => {
                flatten_node(&mut ctx, node);
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
            let idx = flatten_node(&mut multi_ctx, node);
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

        let Some(noise_router_value) = noise_settings
            .get("noise_router")
        else {
            eprintln!("Warning: No noise_router found in {file_path}");
            continue;
        };

        let noise_router: FxHashMap<String, DensityFunctionNode> = match serde_json::from_value(noise_router_value.clone()) {
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
