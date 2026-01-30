use std::fs;

use enum_dispatch::enum_dispatch;
use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::density_function_utils::{DATAPACK_BASE, strip_minecraft_prefix};

pub fn hash_node(value: &DensityFunctionNode) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Context for flattening density functions with reference resolution
pub struct FlattenContext {
    env_prefix: String,
    pub stack: Vec<TokenStream>,
    seen: FxHashMap<String, usize>,
    pub static_data: Vec<TokenStream>,
    data_counter: usize,
    pub splines: Vec<TokenStream>,
    spline_counter: usize,
    /// Cache for resolved density function references
    ref_cache: FxHashMap<String, DensityFunctionNode>,
}

impl FlattenContext {
    pub fn new(env_name: &str) -> Self {
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

    pub fn next_data_name(&mut self, prefix: &str) -> Ident {
        let name = format!("{}_{prefix}_{}", self.env_prefix, self.data_counter);
        self.data_counter += 1;
        Ident::new(&name, Span::call_site())
    }

    pub fn next_spline_name(&mut self) -> Ident {
        let name = format!("{}_SPLINE_{}", self.env_prefix, self.spline_counter);
        self.spline_counter += 1;
        Ident::new(&name, Span::call_site())
    }

    /// Resolve a density function reference like "minecraft:overworld/continents"
    pub fn resolve_reference(&mut self, reference: &str) -> DensityFunctionNode {
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

#[enum_dispatch]
pub trait DensityFunctionTrait {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream;
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(untagged)]
pub enum DensityFunctionNode {
    Constant(f64),
    Reference(String),
    Function(Box<DensityFunction>),
}

impl DensityFunctionNode {
    pub fn flatten(&self, ctx: &mut FlattenContext) -> usize {
        match self {
            DensityFunctionNode::Constant(val) => {
                let hash = format!("const:{val}");
                if let Some(&idx) = ctx.seen.get(&hash) {
                    return idx;
                }
                let component = quote! { BaseNoiseFunctionComponent::Constant { value: #val } };
                let idx = ctx.stack.len();
                ctx.stack.push(component);
                ctx.seen.insert(hash, idx);
                idx
            }
            DensityFunctionNode::Reference(reference) => {
                let resolved = ctx.resolve_reference(reference);
                resolved.flatten(ctx)
            }
            DensityFunctionNode::Function(function) => {
                let hash = hash_node(self);
                if let Some(&idx) = ctx.seen.get(&hash) {
                    return idx;
                }

                let component = function.to_token(ctx);
                let idx = ctx.stack.len();
                ctx.stack.push(component);
                ctx.seen.insert(hash, idx);
                idx
            }
        }
    }
}

#[enum_dispatch(DensityFunctionTrait)]
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum DensityFunction {
    #[serde(rename = "minecraft:blend_offset")]
    BlendOffset(BlendOffset),
    #[serde(rename = "minecraft:blend_alpha")]
    BlendAlpha(BlendAlpha),
    #[serde(rename = "minecraft:end_islands")]
    EndIslands(EndIslands),
    #[serde(rename = "minecraft:beardifier")]
    Beardifier(Beardifier),
    #[serde(rename = "minecraft:old_blended_noise")]
    OldBlendedNoise(OldBlendedNoise),
    #[serde(rename = "minecraft:y_clamped_gradient")]
    YClampedGradient(YClampedGradient),
    #[serde(rename = "minecraft:noise")]
    Noise(Noise),
    #[serde(rename = "minecraft:shift_a")]
    ShiftA(ShiftA),
    #[serde(rename = "minecraft:shift_b")]
    ShiftB(ShiftB),
    #[serde(rename = "minecraft:shifted_noise")]
    ShiftedNoise(ShiftedNoise),
    #[serde(rename = "minecraft:interpolated")]
    Interpolated(Interpolated),
    #[serde(rename = "minecraft:flat_cache")]
    FlatCache(FlatCache),
    #[serde(rename = "minecraft:cache_2d")]
    Cache2D(Cache2D),
    #[serde(rename = "minecraft:cache_once")]
    CacheOnce(CacheOnce),
    #[serde(rename = "minecraft:cache_all_in_cell")]
    CacheAllInCell(CacheAllInCell),
    #[serde(rename = "minecraft:add")]
    Add(Add),
    #[serde(rename = "minecraft:mul")]
    Mul(Mul),
    #[serde(rename = "minecraft:min")]
    Min(Min),
    #[serde(rename = "minecraft:max")]
    Max(Max),
    #[serde(rename = "minecraft:abs")]
    Abs(Abs),
    #[serde(rename = "minecraft:square")]
    Square(Square),
    #[serde(rename = "minecraft:cube")]
    Cube(Cube),
    #[serde(rename = "minecraft:half_negative")]
    HalfNegative(HalfNegative),
    #[serde(rename = "minecraft:quarter_negative")]
    QuarterNegative(QuarterNegative),
    #[serde(rename = "minecraft:squeeze")]
    Squeeze(Squeeze),
    #[serde(rename = "minecraft:clamp")]
    Clamp(Clamp),
    #[serde(rename = "minecraft:range_choice")]
    RangeChoice(RangeChoice),
    #[serde(rename = "minecraft:blend_density")]
    BlendDensity(BlendDensity),
    #[serde(rename = "minecraft:weird_scaled_sampler")]
    WeirdScaledSampler(WeirdScaledSampler),
    #[serde(rename = "minecraft:spline")]
    Spline(Spline),
    #[serde(rename = "minecraft:find_top_surface")]
    FindTopSurface(FindTopSurface),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BlendOffset;

impl DensityFunctionTrait for BlendOffset {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        quote! { BaseNoiseFunctionComponent::BlendOffset }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BlendAlpha;

impl DensityFunctionTrait for BlendAlpha {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        quote! { BaseNoiseFunctionComponent::BlendAlpha }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EndIslands;

impl DensityFunctionTrait for EndIslands {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        quote! { BaseNoiseFunctionComponent::EndIslands }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Beardifier;

impl DensityFunctionTrait for Beardifier {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        quote! { BaseNoiseFunctionComponent::Beardifier }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OldBlendedNoise {
    #[serde(default)]
    pub xz_scale: f64,
    #[serde(default)]
    pub y_scale: f64,
    #[serde(default)]
    pub xz_factor: f64,
    #[serde(default)]
    pub y_factor: f64,
    #[serde(default)]
    pub smear_scale_multiplier: f64,
}

impl DensityFunctionTrait for OldBlendedNoise {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct YClampedGradient {
    from_y: i32,
    to_y: i32,
    from_value: f64,
    to_value: f64,
}

impl DensityFunctionTrait for YClampedGradient {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let from_y = f64::from(self.from_y);
        let to_y = f64::from(self.to_y);
        let from_value = self.from_value;
        let to_value = self.to_value;
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Noise {
    #[serde(rename = "noise")]
    id: String,
    #[serde(default = "default_scale")]
    xz_scale: f64,
    #[serde(default = "default_scale")]
    y_scale: f64,
}

impl DensityFunctionTrait for Noise {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let noise_id = strip_minecraft_prefix(&self.id);
        let xz_scale = self.xz_scale;
        let y_scale = self.y_scale;
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ShiftA {
    argument: String,
}

impl DensityFunctionTrait for ShiftA {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        let noise_id = strip_minecraft_prefix(&self.argument);
        quote! { BaseNoiseFunctionComponent::ShiftA { noise_id: #noise_id } }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ShiftB {
    argument: String,
}

impl DensityFunctionTrait for ShiftB {
    fn to_token(&self, _ctx: &mut FlattenContext) -> TokenStream {
        let noise_id = strip_minecraft_prefix(&self.argument);
        quote! { BaseNoiseFunctionComponent::ShiftB { noise_id: #noise_id } }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ShiftedNoise {
    noise: String,
    #[serde(default = "default_scale")]
    xz_scale: f64,
    #[serde(default = "default_scale")]
    y_scale: f64,
    shift_x: DensityFunctionNode,
    shift_y: DensityFunctionNode,
    shift_z: DensityFunctionNode,
}

impl DensityFunctionTrait for ShiftedNoise {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let noise_id = strip_minecraft_prefix(&self.noise);

        let idx_for_x = self.shift_x.flatten(ctx);
        let idx_for_y = self.shift_y.flatten(ctx);
        let idx_for_z = self.shift_z.flatten(ctx);

        let xz_scale = self.xz_scale;
        let y_scale = self.y_scale;

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
}

fn handle_cache(
    ctx: &mut FlattenContext,
    wrapped_node: &DensityFunctionNode,
    variant: TokenStream,
) -> TokenStream {
    let input_idx = wrapped_node.flatten(ctx);

    quote! {
        BaseNoiseFunctionComponent::Wrapper {
            input_index: #input_idx,
            wrapper: #variant,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Interpolated {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Interpolated {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_cache(ctx, &self.argument, quote! { WrapperType::Interpolated })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FlatCache {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for FlatCache {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_cache(ctx, &self.argument, quote! { WrapperType::CacheFlat })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Cache2D {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Cache2D {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_cache(ctx, &self.argument, quote! { WrapperType::Cache2D })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CacheOnce {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for CacheOnce {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_cache(ctx, &self.argument, quote! { WrapperType::CacheOnce })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CacheAllInCell {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for CacheAllInCell {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_cache(ctx, &self.argument, quote! { WrapperType::CellCache })
    }
}

fn handle_binary_operation(
    ctx: &mut FlattenContext,
    arg1: &DensityFunctionNode,
    arg2: &DensityFunctionNode,
    op: TokenStream,
) -> TokenStream {
    let idx_arg1 = arg1.flatten(ctx);
    let idx_arg2 = arg2.flatten(ctx);

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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Add {
    argument1: DensityFunctionNode,
    argument2: DensityFunctionNode,
}

impl DensityFunctionTrait for Add {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_binary_operation(
            ctx,
            &self.argument1,
            &self.argument2,
            quote! { BinaryOperation::Add },
        )
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Mul {
    argument1: DensityFunctionNode,
    argument2: DensityFunctionNode,
}

impl DensityFunctionTrait for Mul {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_binary_operation(
            ctx,
            &self.argument1,
            &self.argument2,
            quote! { BinaryOperation::Mul },
        )
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Min {
    argument1: DensityFunctionNode,
    argument2: DensityFunctionNode,
}

impl DensityFunctionTrait for Min {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_binary_operation(
            ctx,
            &self.argument1,
            &self.argument2,
            quote! { BinaryOperation::Min },
        )
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Max {
    argument1: DensityFunctionNode,
    argument2: DensityFunctionNode,
}

impl DensityFunctionTrait for Max {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_binary_operation(
            ctx,
            &self.argument1,
            &self.argument2,
            quote! { BinaryOperation::Max },
        )
    }
}

fn handle_unary_operation(
    ctx: &mut FlattenContext,
    input: &DensityFunctionNode,
    op: TokenStream,
) -> TokenStream {
    let input_idx = input.flatten(ctx);

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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Abs {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Abs {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(ctx, &self.argument, quote! { UnaryOperation::Abs })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Square {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Square {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(ctx, &self.argument, quote! { UnaryOperation::Square })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Cube {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Cube {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(ctx, &self.argument, quote! { UnaryOperation::Cube })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct HalfNegative {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for HalfNegative {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(ctx, &self.argument, quote! { UnaryOperation::HalfNegative })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct QuarterNegative {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for QuarterNegative {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(
            ctx,
            &self.argument,
            quote! { UnaryOperation::QuarterNegative },
        )
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Squeeze {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for Squeeze {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        handle_unary_operation(ctx, &self.argument, quote! { UnaryOperation::Squeeze })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Clamp {
    input: DensityFunctionNode,
    min: f64,
    max: f64,
}

impl DensityFunctionTrait for Clamp {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let input_idx = self.input.flatten(ctx);
        let min_val = self.min;
        let max_val = self.max;

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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RangeChoice {
    input: DensityFunctionNode,
    min_inclusive: f64,
    max_exclusive: f64,
    when_in_range: DensityFunctionNode,
    when_out_of_range: DensityFunctionNode,
}

impl DensityFunctionTrait for RangeChoice {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let input_idx = self.input.flatten(ctx);
        let when_in_idx = self.when_in_range.flatten(ctx);
        let when_out_idx = self.when_out_of_range.flatten(ctx);
        let min_inclusive = self.min_inclusive;
        let max_exclusive = self.max_exclusive;

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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BlendDensity {
    argument: DensityFunctionNode,
}

impl DensityFunctionTrait for BlendDensity {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let input_idx = self.argument.flatten(ctx);
        quote! { BaseNoiseFunctionComponent::BlendDensity { input_index: #input_idx } }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WeirdScaledSampler {
    input: DensityFunctionNode,
    rarity_value_mapper: String,
    noise: String,
}

impl DensityFunctionTrait for WeirdScaledSampler {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let noise_id = strip_minecraft_prefix(&self.noise);
        let input_idx = self.input.flatten(ctx);

        let mapper = match &self.rarity_value_mapper as &str {
            "type_1" => quote! { WeirdScaledMapper::Tunnels },
            "type_2" => quote! { WeirdScaledMapper::Caves },
            _ => panic!("Unknown rarity mapper: {}", self.rarity_value_mapper),
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Spline {
    spline: SplineData,
}

impl DensityFunctionTrait for Spline {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        let spline_name = self.spline.flatten(ctx);
        quote! { BaseNoiseFunctionComponent::Spline { spline: &#spline_name } }
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FindTopSurface {
    pub density: Option<DensityFunctionNode>,
}

impl DensityFunctionTrait for FindTopSurface {
    fn to_token(&self, ctx: &mut FlattenContext) -> TokenStream {
        if let Some(d) = &self.density {
            d.flatten(ctx);
        }
        quote! { BaseNoiseFunctionComponent::Constant { value: 0.0 } }
    }
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

impl SplineData {
    fn flatten(&self, ctx: &mut FlattenContext) -> Ident {
        let spline_name = ctx.next_spline_name();

        match self {
            SplineData::Fixed(val) => {
                let value_f32 = *val as f32;
                ctx.splines.push(quote! {
                    static #spline_name: SplineRepr = SplineRepr::Fixed { value: #value_f32 };
                });
                spline_name
            }
            SplineData::Spline { coordinate, points } => {
                let coord_idx = coordinate.flatten(ctx);

                let mut point_tokens = Vec::new();

                for point in points {
                    let location = point.location as f32;
                    let derivative = point.derivative as f32;
                    let value_spline_name = point.value.flatten(ctx);

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
                spline_name
            }
        }
    }
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
