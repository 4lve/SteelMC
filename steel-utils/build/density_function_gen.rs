//! Build-time code generator for density functions.

use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;
use serde_json::{Map, Value};

struct FlattenContext {
    env_prefix: String,
    stack: Vec<TokenStream>,
    seen: FxHashMap<String, usize>,
    static_data: Vec<TokenStream>,
    data_counter: usize,
    splines: Vec<TokenStream>,
    spline_counter: usize,
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
}

fn hash_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn get_f64(val: &Value, key: &str) -> f64 {
    val.get(key)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn get_str<'a>(val: &'a Value, key: &str) -> &'a str {
    val.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn strip_minecraft_prefix(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

fn handle_blended_noise(ctx: &mut FlattenContext) -> TokenStream {
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

fn handle_constant(value: &Value) -> TokenStream {
    let val = value
        .get("value")
        .and_then(Value::as_f64)
        .expect("Missing constant value");
    quote! { BaseNoiseFunctionComponent::Constant { value: #val } }
}

fn handle_y_clamped_gradient(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let from_y = get_f64(value, "fromY");
    let to_y = get_f64(value, "toY");
    let from_value = get_f64(value, "fromValue");
    let to_value = get_f64(value, "toValue");

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

fn handle_noise(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "noise"));
    let xz_scale = get_f64(value, "xzScale");
    let y_scale = get_f64(value, "yScale");

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

fn handle_shift(value: &Value, is_shift_a: bool) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "offsetNoise"));
    if is_shift_a {
        quote! { BaseNoiseFunctionComponent::ShiftA { noise_id: #noise_id } }
    } else {
        quote! { BaseNoiseFunctionComponent::ShiftB { noise_id: #noise_id } }
    }
}

fn handle_shifted_noise(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "noise"));
    let xz_scale = get_f64(value, "xzScale");
    let y_scale = get_f64(value, "yScale");

    let idx_for_x = flatten_node(ctx, value.get("shiftX").expect("Missing shiftX"));
    let idx_for_y = flatten_node(ctx, value.get("shiftY").expect("Missing shiftY"));
    let idx_for_z = flatten_node(ctx, value.get("shiftZ").expect("Missing shiftZ"));

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

fn handle_wrapping(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let wrap_type = get_str(value, "type");
    let wrapped_node = value.get("wrapped").expect("Missing wrapped");
    let input_idx = flatten_node(ctx, wrapped_node);

    let wrap_variant = match wrap_type {
        "Interpolated" => quote! { WrapperType::Interpolated },
        "FlatCache" => quote! { WrapperType::CacheFlat },
        "Cache2D" => quote! { WrapperType::Cache2D },
        "CacheOnce" => quote! { WrapperType::CacheOnce },
        "CellCache" => quote! { WrapperType::CellCache },
        _ => panic!("Unknown wrapper type: {wrap_type}"),
    };

    quote! {
        BaseNoiseFunctionComponent::Wrapper {
            input_index: #input_idx,
            wrapper: #wrap_variant,
        }
    }
}

fn handle_binary_operation(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let operation = get_str(value, "type");
    let arg1 = value.get("argument1").expect("Missing argument1");
    let arg2 = value.get("argument2").expect("Missing argument2");

    let idx_arg1 = flatten_node(ctx, arg1);
    let idx_arg2 = flatten_node(ctx, arg2);

    let op = match operation {
        "ADD" => quote! { BinaryOperation::Add },
        "MUL" => quote! { BinaryOperation::Mul },
        "MIN" => quote! { BinaryOperation::Min },
        "MAX" => quote! { BinaryOperation::Max },
        _ => panic!("Unknown binary operation: {operation}"),
    };

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

fn handle_unary_operation(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let operation = get_str(value, "type");
    let input = value.get("input").expect("Missing input");
    let input_idx = flatten_node(ctx, input);

    let op = match operation {
        "ABS" => quote! { UnaryOperation::Abs },
        "SQUARE" => quote! { UnaryOperation::Square },
        "CUBE" => quote! { UnaryOperation::Cube },
        "HALF_NEGATIVE" => quote! { UnaryOperation::HalfNegative },
        "QUARTER_NEGATIVE" => quote! { UnaryOperation::QuarterNegative },
        "SQUEEZE" => quote! { UnaryOperation::Squeeze },
        _ => panic!("Unknown unary operation: {operation}"),
    };

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

fn handle_clamp(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let min_val = get_f64(value, "minValue");
    let max_val = get_f64(value, "maxValue");

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

fn handle_range_choice(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let when_in_range = value.get("whenInRange").expect("Missing whenInRange");
    let when_out_of_range = value.get("whenOutOfRange").expect("Missing whenOutOfRange");
    let min_inclusive = get_f64(value, "minInclusive");
    let max_exclusive = get_f64(value, "maxExclusive");

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

fn handle_blend_density(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let input_idx = flatten_node(ctx, input);
    quote! { BaseNoiseFunctionComponent::BlendDensity { input_index: #input_idx } }
}

fn handle_weird_scaled_sampler(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let noise_id = strip_minecraft_prefix(get_str(value, "noise"));
    let rarity_mapper = get_str(value, "rarityValueMapper");

    let input_idx = flatten_node(ctx, input);

    let mapper = match rarity_mapper {
        "TYPE1" => quote! { WeirdScaledMapper::Tunnels },
        "TYPE2" => quote! { WeirdScaledMapper::Caves },
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

fn handle_spline_component(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let spline = value.get("spline").expect("Missing spline");
    let spline_name = flatten_spline(ctx, spline);
    quote! { BaseNoiseFunctionComponent::Spline { spline: &#spline_name } }
}

fn handle_linear_operation(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let operation = get_str(value, "type");
    let argument = get_f64(value, "argument");

    let input_idx = flatten_node(ctx, input);

    let op = match operation {
        "ADD" => quote! { LinearOperation::Add },
        "MUL" => quote! { LinearOperation::Mul },
        _ => panic!("Unknown linear operation: {operation}"),
    };

    let data_name = ctx.next_data_name("LINEAR_DATA");
    ctx.static_data.push(quote! {
        static #data_name: LinearData = LinearData {
            operation: #op,
            argument: #argument,
        };
    });

    quote! {
        BaseNoiseFunctionComponent::Linear {
            input_index: #input_idx,
            data: &#data_name,
        }
    }
}

fn flatten_node(ctx: &mut FlattenContext, node: &Value) -> usize {
    let hash = hash_json(node);
    if let Some(&idx) = ctx.seen.get(&hash) {
        return idx;
    }

    let obj = node
        .as_object()
        .unwrap_or_else(|| panic!("Expected object node, got: {node:?}"));
    let class = obj
        .get("_class")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Missing _class in node: {obj:?}"));

    let value = obj.get("value");

    let component = match class {
        "BlendOffset" => quote! { BaseNoiseFunctionComponent::BlendOffset },
        "BlendAlpha" => quote! { BaseNoiseFunctionComponent::BlendAlpha },
        "EndIslands" => quote! { BaseNoiseFunctionComponent::EndIslands },
        "FindTopSurface" => quote! { BaseNoiseFunctionComponent::FindTopSurface },
        "BlendedNoise" => handle_blended_noise(ctx),
        "Constant" => handle_constant(value.expect("Missing value for Constant")),
        "YClampedGradient" => {
            handle_y_clamped_gradient(ctx, value.expect("Missing value for YClampedGradient"))
        }
        "Noise" => handle_noise(ctx, value.expect("Missing value for Noise")),
        "ShiftA" => handle_shift(value.expect("Missing value for ShiftA"), true),
        "ShiftB" => handle_shift(value.expect("Missing value for ShiftB"), false),
        "ShiftedNoise" => handle_shifted_noise(ctx, value.expect("Missing value for ShiftedNoise")),
        "Wrapping" => handle_wrapping(ctx, value.expect("Missing value for Wrapping")),
        "BinaryOperation" => {
            handle_binary_operation(ctx, value.expect("Missing value for BinaryOperation"))
        }
        "UnaryOperation" => {
            handle_unary_operation(ctx, value.expect("Missing value for UnaryOperation"))
        }
        "Clamp" => handle_clamp(ctx, value.expect("Missing value for Clamp")),
        "RangeChoice" => handle_range_choice(ctx, value.expect("Missing value for RangeChoice")),
        "BlendDensity" => handle_blend_density(ctx, value.expect("Missing value for BlendDensity")),
        "WeirdScaledSampler" => {
            handle_weird_scaled_sampler(ctx, value.expect("Missing value for WeirdScaledSampler"))
        }
        "Spline" => handle_spline_component(ctx, value.expect("Missing value for Spline")),
        "LinearOperation" => {
            handle_linear_operation(ctx, value.expect("Missing value for LinearOperation"))
        }
        _ => panic!("Unknown density function class: {class}"),
    };

    let idx = ctx.stack.len();
    ctx.stack.push(component);
    ctx.seen.insert(hash, idx);
    idx
}

fn flatten_spline(ctx: &mut FlattenContext, spline: &Value) -> Ident {
    let spline_name = ctx.next_spline_name();

    let obj = spline.as_object().expect("Spline should be object");
    let spline_type = obj
        .get("_type")
        .and_then(Value::as_str)
        .expect("Missing _type");

    match spline_type {
        "fixed" => {
            let value_obj = obj.get("value").expect("Missing value in fixed spline");
            let value = value_obj
                .get("value")
                .and_then(Value::as_f64)
                .expect("Missing value.value") as f32;
            ctx.splines.push(quote! {
                static #spline_name: SplineRepr = SplineRepr::Fixed { value: #value };
            });
        }
        "standard" => {
            let value_obj = obj.get("value").expect("Missing value in standard spline");
            let location_fn = value_obj
                .get("locationFunction")
                .expect("Missing locationFunction");
            let locations = value_obj
                .get("locations")
                .and_then(Value::as_array)
                .expect("Missing locations");
            let values = value_obj
                .get("values")
                .and_then(Value::as_array)
                .expect("Missing values");
            let derivatives = value_obj
                .get("derivatives")
                .and_then(Value::as_array)
                .expect("Missing derivatives");

            let coord_idx = flatten_node(ctx, location_fn);

            let mut point_tokens = Vec::new();

            for ((location_val, value_spline), derivative_val) in
                locations.iter().zip(values.iter()).zip(derivatives.iter())
            {
                let location = location_val.as_f64().expect("location should be f64") as f32;
                let derivative = derivative_val.as_f64().expect("derivative should be f64") as f32;
                let value_spline_name = flatten_spline(ctx, value_spline);

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
        _ => panic!("Unknown spline type: {spline_type}"),
    }

    spline_name
}

fn generate_environment(env_name: &str, env_data: &Map<String, Value>) -> (TokenStream, String) {
    let mut ctx = FlattenContext::new(env_name);

    let field_mapping = [
        ("barrierNoise", "barrier_noise"),
        (
            "fluidLevelFloodednessNoise",
            "fluid_level_floodedness_noise",
        ),
        ("fluidLevelSpreadNoise", "fluid_level_spread_noise"),
        ("lavaNoise", "lava_noise"),
        ("erosion", "erosion"),
        ("depth", "depth"),
        ("finalDensity", "final_density"),
        ("veinToggle", "vein_toggle"),
        ("veinRidged", "vein_ridged"),
        ("veinGap", "vein_gap"),
        ("temperature", "temperature"),
        ("vegetation", "vegetation"),
        ("continents", "continents"),
        ("ridges", "ridges"),
    ];

    let mut indices: FxHashMap<&str, usize> = FxHashMap::default();

    for (json_name, _rust_name) in &field_mapping {
        if let Some(node) = env_data.get(*json_name) {
            let idx = flatten_node(&mut ctx, node);
            indices.insert(json_name, idx);
        }
    }

    let components = &ctx.stack;
    let static_data = &ctx.static_data;
    let splines = &ctx.splines;

    let env_name_upper = env_name.to_shouty_snake_case();
    let stack_name = Ident::new(
        &format!("{env_name_upper}_COMPONENT_STACK"),
        Span::call_site(),
    );
    let router_name = Ident::new(
        &format!("{env_name_upper}_BASE_NOISE_ROUTER"),
        Span::call_site(),
    );

    let barrier_noise = indices.get("barrierNoise").copied().unwrap_or(0);
    let fluid_floodedness = indices
        .get("fluidLevelFloodednessNoise")
        .copied()
        .unwrap_or(0);
    let fluid_spread = indices.get("fluidLevelSpreadNoise").copied().unwrap_or(0);
    let lava_noise = indices.get("lavaNoise").copied().unwrap_or(0);
    let erosion = indices.get("erosion").copied().unwrap_or(0);
    let depth = indices.get("depth").copied().unwrap_or(0);
    let final_density = indices.get("finalDensity").copied().unwrap_or(0);
    let vein_toggle = indices.get("veinToggle").copied().unwrap_or(0);
    let vein_ridged = indices.get("veinRidged").copied().unwrap_or(0);
    let vein_gap = indices.get("veinGap").copied().unwrap_or(0);

    let components_len = components.len();

    let stream = quote! {
        #(#static_data)*

        #(#splines)*

        static #stack_name: [BaseNoiseFunctionComponent; #components_len] = [
            #(#components),*
        ];

        pub static #router_name: BaseNoiseRouters = BaseNoiseRouters {
            noise: BaseNoiseRouter {
                full_component_stack: &#stack_name,
                barrier_noise: #barrier_noise,
                fluid_level_floodedness_noise: #fluid_floodedness,
                fluid_level_spread_noise: #fluid_spread,
                lava_noise: #lava_noise,
                erosion: #erosion,
                depth: #depth,
                final_density: #final_density,
                vein_toggle: #vein_toggle,
                vein_ridged: #vein_ridged,
                vein_gap: #vein_gap,
            },
            surface_estimator: BaseSurfaceEstimator {
                full_component_stack: &[],
            },
            multi_noise: BaseMultiNoiseRouter {
                full_component_stack: &[],
                temperature: 0,
                vegetation: 0,
                continents: 0,
                erosion: 0,
                depth: 0,
                ridges: 0,
            },
        };
    };

    (stream, router_name.to_string())
}

pub(crate) fn build() -> TokenStream {
    println!("cargo:rerun-if-changed=build_assets/density_function.json");

    let json_file = fs::read_to_string("build_assets/density_function.json")
        .expect("Failed to read density_function.json");

    let json_file = json_file
        .replace("-Infinity", "\"__NEG_INFINITY__\"")
        .replace("Infinity", "\"__POS_INFINITY__\"")
        .replace("NaN", "\"__NAN__\"");

    let data: Map<String, Value> =
        serde_json::from_str(&json_file).expect("Failed to parse density_function.json");

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::noise_router::data::{
            BaseNoiseFunctionComponent, BaseNoiseRouter, BaseNoiseRouters, BaseSurfaceEstimator,
            BaseMultiNoiseRouter, NoiseData, ShiftedNoiseData, ClampedYGradientData,
            BinaryData, BinaryOperation, UnaryData, UnaryOperation, LinearData, LinearOperation,
            ClampData, RangeChoiceData, WeirdScaledData, WeirdScaledMapper,
            InterpolatedNoiseSamplerData, WrapperType, SplineRepr, SplinePoint,
        };
    });

    let environments = [
        "overworld",
        "amplified",
        "large_biomes",
        "nether",
        "end",
        "caves",
        "floating_islands",
    ];

    for env_name in environments {
        if let Some(env_data) = data.get(env_name).and_then(Value::as_object) {
            let (env_stream, _router_name) = generate_environment(env_name, env_data);
            stream.extend(env_stream);
        }
    }

    stream
}
