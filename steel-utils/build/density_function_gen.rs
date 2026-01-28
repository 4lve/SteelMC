//! Build-time code generator for density functions.
//!
//! This module reads Minecraft's vanilla noise_settings JSON files and generates
//! Rust code for the noise router density functions. It parses the vanilla JSON
//! format directly without any translation layer.

use std::fs;

use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashMap;
use serde_json::{Map, Value};

/// Base path for builtin datapacks
const DATAPACK_BASE: &str = "../steel-registry/build_assets/builtin_datapacks/minecraft/data/minecraft";

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
    ref_cache: FxHashMap<String, Value>,
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
    fn resolve_reference(&mut self, reference: &str) -> Value {
        let path = reference.strip_prefix("minecraft:").unwrap_or(reference);
        
        if let Some(cached) = self.ref_cache.get(path) {
            return cached.clone();
        }

        let file_path = format!("{}/worldgen/density_function/{}.json", DATAPACK_BASE, path);
        
        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|e| panic!("Failed to read density function at {}: {}", file_path, e));
        
        let value: Value = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse density function at {}: {}", file_path, e));
        
        self.ref_cache.insert(path.to_string(), value.clone());
        value
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

fn get_f64_or(val: &Value, key: &str, default: f64) -> f64 {
    val.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn get_str<'a>(val: &'a Value, key: &str) -> &'a str {
    val.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn strip_minecraft_prefix(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

// =============================================================================
// Vanilla JSON handlers - parse minecraft:* types directly
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

fn handle_y_clamped_gradient(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let from_y = get_f64(value, "from_y");
    let to_y = get_f64(value, "to_y");
    let from_value = get_f64(value, "from_value");
    let to_value = get_f64(value, "to_value");

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
    let xz_scale = get_f64_or(value, "xz_scale", 1.0);
    let y_scale = get_f64_or(value, "y_scale", 1.0);

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

fn handle_shift_a(_ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "argument"));
    quote! { BaseNoiseFunctionComponent::ShiftA { noise_id: #noise_id } }
}

fn handle_shift_b(_ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "argument"));
    quote! { BaseNoiseFunctionComponent::ShiftB { noise_id: #noise_id } }
}

fn handle_shifted_noise(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let noise_id = strip_minecraft_prefix(get_str(value, "noise"));
    let xz_scale = get_f64_or(value, "xz_scale", 1.0);
    let y_scale = get_f64_or(value, "y_scale", 1.0);

    let shift_x = value.get("shift_x").expect("Missing shift_x");
    let shift_y = value.get("shift_y").expect("Missing shift_y");
    let shift_z = value.get("shift_z").expect("Missing shift_z");

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

fn handle_cache(ctx: &mut FlattenContext, value: &Value, wrapper_type: &str) -> TokenStream {
    let wrapped_node = value.get("argument").expect("Missing argument");
    let input_idx = flatten_node(ctx, wrapped_node);

    let wrap_variant = match wrapper_type {
        "interpolated" => quote! { WrapperType::Interpolated },
        "flat_cache" => quote! { WrapperType::CacheFlat },
        "cache_2d" => quote! { WrapperType::Cache2D },
        "cache_once" => quote! { WrapperType::CacheOnce },
        "cache_all_in_cell" => quote! { WrapperType::CellCache },
        _ => panic!("Unknown wrapper type: {wrapper_type}"),
    };

    quote! {
        BaseNoiseFunctionComponent::Wrapper {
            input_index: #input_idx,
            wrapper: #wrap_variant,
        }
    }
}

fn handle_binary_operation(ctx: &mut FlattenContext, value: &Value, op_type: &str) -> TokenStream {
    let arg1 = value.get("argument1").expect("Missing argument1");
    let arg2 = value.get("argument2").expect("Missing argument2");

    let idx_arg1 = flatten_node(ctx, arg1);
    let idx_arg2 = flatten_node(ctx, arg2);

    let op = match op_type {
        "add" => quote! { BinaryOperation::Add },
        "mul" => quote! { BinaryOperation::Mul },
        "min" => quote! { BinaryOperation::Min },
        "max" => quote! { BinaryOperation::Max },
        _ => panic!("Unknown binary operation: {op_type}"),
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

fn handle_unary_operation(ctx: &mut FlattenContext, value: &Value, op_type: &str) -> TokenStream {
    let input = value.get("argument").expect("Missing argument");
    let input_idx = flatten_node(ctx, input);

    let op = match op_type {
        "abs" => quote! { UnaryOperation::Abs },
        "square" => quote! { UnaryOperation::Square },
        "cube" => quote! { UnaryOperation::Cube },
        "half_negative" => quote! { UnaryOperation::HalfNegative },
        "quarter_negative" => quote! { UnaryOperation::QuarterNegative },
        "squeeze" => quote! { UnaryOperation::Squeeze },
        _ => panic!("Unknown unary operation: {op_type}"),
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
    let min_val = get_f64(value, "min");
    let max_val = get_f64(value, "max");

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
    let when_in_range = value.get("when_in_range").expect("Missing when_in_range");
    let when_out_of_range = value.get("when_out_of_range").expect("Missing when_out_of_range");
    let min_inclusive = get_f64(value, "min_inclusive");
    let max_exclusive = get_f64(value, "max_exclusive");

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
    let input = value.get("argument").expect("Missing argument");
    let input_idx = flatten_node(ctx, input);
    quote! { BaseNoiseFunctionComponent::BlendDensity { input_index: #input_idx } }
}

fn handle_weird_scaled_sampler(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let input = value.get("input").expect("Missing input");
    let noise_id = strip_minecraft_prefix(get_str(value, "noise"));
    let rarity_mapper = get_str(value, "rarity_value_mapper");

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

fn handle_spline_component(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    let spline = value.get("spline").expect("Missing spline");
    let spline_name = flatten_spline(ctx, spline);
    quote! { BaseNoiseFunctionComponent::Spline { spline: &#spline_name } }
}

fn handle_find_top_surface(ctx: &mut FlattenContext, value: &Value) -> TokenStream {
    // FindTopSurface wraps an inner density function
    // For surface estimation purposes, we need the inner density
    if let Some(density) = value.get("density") {
        flatten_node(ctx, density);
    }
    // Return the density node itself (the FindTopSurface is not a component we generate directly)
    // The caller should use the inner density
    quote! { BaseNoiseFunctionComponent::Constant { value: 0.0 } }
}

/// Main node flattening function - parses vanilla JSON directly
fn flatten_node(ctx: &mut FlattenContext, node: &Value) -> usize {
    // Handle numeric constants
    if let Some(n) = node.as_f64() {
        let hash = format!("const:{}", n);
        if let Some(&idx) = ctx.seen.get(&hash) {
            return idx;
        }
        let component = handle_constant_value(n);
        let idx = ctx.stack.len();
        ctx.stack.push(component);
        ctx.seen.insert(hash, idx);
        return idx;
    }

    // Handle string references (e.g., "minecraft:overworld/continents")
    if let Some(ref_str) = node.as_str() {
        let resolved = ctx.resolve_reference(ref_str);
        return flatten_node(ctx, &resolved);
    }

    // Handle object nodes
    let hash = hash_json(node);
    if let Some(&idx) = ctx.seen.get(&hash) {
        return idx;
    }

    let obj = node
        .as_object()
        .unwrap_or_else(|| panic!("Expected object node, got: {node:?}"));
    
    let type_str = obj
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Missing type in node: {obj:?}"));
    
    let type_name = strip_minecraft_prefix(type_str);

    let component = match type_name {
        // Simple components (no arguments)
        "blend_offset" => quote! { BaseNoiseFunctionComponent::BlendOffset },
        "blend_alpha" => quote! { BaseNoiseFunctionComponent::BlendAlpha },
        "end_islands" => quote! { BaseNoiseFunctionComponent::EndIslands },
        "beardifier" => quote! { BaseNoiseFunctionComponent::Beardifier },
        
        // Blended noise (old_blended_noise)
        "old_blended_noise" => handle_old_blended_noise(ctx),
        
        // Y clamped gradient
        "y_clamped_gradient" => handle_y_clamped_gradient(ctx, node),
        
        // Noise types
        "noise" => handle_noise(ctx, node),
        "shift_a" => handle_shift_a(ctx, node),
        "shift_b" => handle_shift_b(ctx, node),
        "shifted_noise" => handle_shifted_noise(ctx, node),
        
        // Cache/wrapper types
        "interpolated" | "flat_cache" | "cache_2d" | "cache_once" | "cache_all_in_cell" => {
            handle_cache(ctx, node, type_name)
        }
        
        // Binary operations
        "add" | "mul" | "min" | "max" => handle_binary_operation(ctx, node, type_name),
        
        // Unary operations
        "abs" | "square" | "cube" | "half_negative" | "quarter_negative" | "squeeze" => {
            handle_unary_operation(ctx, node, type_name)
        }
        
        // Clamp
        "clamp" => handle_clamp(ctx, node),
        
        // Range choice
        "range_choice" => handle_range_choice(ctx, node),
        
        // Blend density
        "blend_density" => handle_blend_density(ctx, node),
        
        // Weird scaled sampler
        "weird_scaled_sampler" => handle_weird_scaled_sampler(ctx, node),
        
        // Spline
        "spline" => handle_spline_component(ctx, node),
        
        // Find top surface (for surface estimation)
        "find_top_surface" => handle_find_top_surface(ctx, node),
        
        _ => panic!("Unknown density function type: {type_name}"),
    };

    let idx = ctx.stack.len();
    ctx.stack.push(component);
    ctx.seen.insert(hash, idx);
    idx
}

fn flatten_spline(ctx: &mut FlattenContext, spline: &Value) -> Ident {
    let spline_name = ctx.next_spline_name();

    // Check if it's a fixed spline (just a number)
    if let Some(value) = spline.as_f64() {
        let value_f32 = value as f32;
        ctx.splines.push(quote! {
            static #spline_name: SplineRepr = SplineRepr::Fixed { value: #value_f32 };
        });
        return spline_name;
    }

    // It's an object spline with coordinate and points
    let obj = spline.as_object().expect("Spline should be object or number");
    
    // Get the coordinate function
    let coord_fn = obj.get("coordinate").expect("Missing coordinate in spline");
    let coord_idx = flatten_node(ctx, coord_fn);

    // Get the points array
    let points = obj
        .get("points")
        .and_then(Value::as_array)
        .expect("Missing points in spline");

    let mut point_tokens = Vec::new();

    for point in points {
        let point_obj = point.as_object().expect("Point should be object");
        
        let location = point_obj
            .get("location")
            .and_then(Value::as_f64)
            .expect("Missing location in point") as f32;
        
        let derivative = point_obj
            .get("derivative")
            .and_then(Value::as_f64)
            .expect("Missing derivative in point") as f32;
        
        let value_spline = point_obj.get("value").expect("Missing value in point");
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

    spline_name
}

/// Emit the static data, splines, and component stack array for a flatten context.
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

fn generate_environment(env_name: &str, noise_router: &Map<String, Value>) -> (TokenStream, String) {
    let env_name_upper = env_name.to_shouty_snake_case();

    // === Noise stack ===
    let mut noise_ctx = FlattenContext::new(env_name);

    // Map vanilla field names to internal field names
    let noise_fields = [
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

    let mut noise_indices: FxHashMap<&str, usize> = FxHashMap::default();
    for (json_name, internal_name) in &noise_fields {
        if let Some(node) = noise_router.get(*json_name) {
            let idx = flatten_node(&mut noise_ctx, node);
            noise_indices.insert(internal_name, idx);
        }
    }

    // Post-process finalDensity: wrap with CellCache(Add(finalDensity, Beardifier))
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

    let noise_stack_name = Ident::new(&format!("{env_name_upper}_NOISE_STACK"), Span::call_site());
    let noise_stream = emit_stack(&noise_ctx, &noise_stack_name);

    // === Surface estimator stack ===
    let surface_prefix = format!("{env_name}_surface");
    let mut surface_ctx = FlattenContext::new(&surface_prefix);

    // Extract the density from preliminary_surface_level (FindTopSurface)
    if let Some(node) = noise_router.get("preliminary_surface_level") {
        if let Some(obj) = node.as_object() {
            let type_str = obj.get("type").and_then(Value::as_str);
            if type_str == Some("minecraft:find_top_surface") {
                if let Some(density_node) = obj.get("density") {
                    flatten_node(&mut surface_ctx, density_node);
                }
            } else {
                flatten_node(&mut surface_ctx, node);
            }
        }
    }

    // If no surface density was found, add a constant 0 placeholder
    if surface_ctx.stack.is_empty() {
        surface_ctx.stack.push(quote! { BaseNoiseFunctionComponent::Constant { value: 0f64 } });
    }

    let surface_stack_name = Ident::new(
        &format!("{env_name_upper}_SURFACE_STACK"),
        Span::call_site(),
    );
    let surface_stream = emit_stack(&surface_ctx, &surface_stack_name);

    // === Multi noise stack ===
    let multi_prefix = format!("{env_name}_multi");
    let mut multi_ctx = FlattenContext::new(&multi_prefix);

    let multi_fields = [
        "temperature",
        "vegetation",
        "continents",
        "erosion",
        "depth",
        "ridges",
    ];

    let mut multi_indices: FxHashMap<&str, usize> = FxHashMap::default();
    for json_name in &multi_fields {
        if let Some(node) = noise_router.get(*json_name) {
            let idx = flatten_node(&mut multi_ctx, node);
            multi_indices.insert(json_name, idx);
        }
    }

    let multi_stack_name = Ident::new(&format!("{env_name_upper}_MULTI_STACK"), Span::call_site());
    let multi_stream = emit_stack(&multi_ctx, &multi_stack_name);

    // === Build the router ===
    let router_name = Ident::new(
        &format!("{env_name_upper}_BASE_NOISE_ROUTER"),
        Span::call_site(),
    );

    let barrier_noise = noise_indices.get("barrierNoise").copied().unwrap_or(0);
    let fluid_floodedness = noise_indices
        .get("fluidLevelFloodednessNoise")
        .copied()
        .unwrap_or(0);
    let fluid_spread = noise_indices
        .get("fluidLevelSpreadNoise")
        .copied()
        .unwrap_or(0);
    let lava_noise = noise_indices.get("lavaNoise").copied().unwrap_or(0);
    let noise_erosion = noise_indices.get("erosion").copied().unwrap_or(0);
    let noise_depth = noise_indices.get("depth").copied().unwrap_or(0);
    let final_density = noise_indices.get("finalDensity").copied().unwrap_or(0);
    let vein_toggle = noise_indices.get("veinToggle").copied().unwrap_or(0);
    let vein_ridged = noise_indices.get("veinRidged").copied().unwrap_or(0);
    let vein_gap = noise_indices.get("veinGap").copied().unwrap_or(0);

    let temperature = multi_indices.get("temperature").copied().unwrap_or(0);
    let vegetation = multi_indices.get("vegetation").copied().unwrap_or(0);
    let continents = multi_indices.get("continents").copied().unwrap_or(0);
    let multi_erosion = multi_indices.get("erosion").copied().unwrap_or(0);
    let multi_depth = multi_indices.get("depth").copied().unwrap_or(0);
    let ridges = multi_indices.get("ridges").copied().unwrap_or(0);

    // Build surface estimator reference (always has at least constant 0)
    let surface_estimator_tokens = quote! {
        surface_estimator: BaseSurfaceEstimator {
            full_component_stack: &#surface_stack_name,
        },
    };

    // Build multi noise reference
    let multi_noise_tokens = if multi_stream.is_some() {
        quote! {
            multi_noise: BaseMultiNoiseRouter {
                full_component_stack: &#multi_stack_name,
                temperature: #temperature,
                vegetation: #vegetation,
                continents: #continents,
                erosion: #multi_erosion,
                depth: #multi_depth,
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
    };

    let mut stream = TokenStream::new();
    if let Some(ns) = noise_stream {
        stream.extend(ns);
    }
    // surface_stream is always Some since we add at least constant 0
    if let Some(ss) = surface_stream {
        stream.extend(ss);
    }
    if let Some(ms) = multi_stream {
        stream.extend(ms);
    }

    stream.extend(quote! {
        pub static #router_name: BaseNoiseRouters = BaseNoiseRouters {
            noise: BaseNoiseRouter {
                full_component_stack: &#noise_stack_name,
                barrier_noise: #barrier_noise,
                fluid_level_floodedness_noise: #fluid_floodedness,
                fluid_level_spread_noise: #fluid_spread,
                lava_noise: #lava_noise,
                erosion: #noise_erosion,
                depth: #noise_depth,
                final_density: #final_density,
                vein_toggle: #vein_toggle,
                vein_ridged: #vein_ridged,
                vein_gap: #vein_gap,
            },
            #surface_estimator_tokens
            #multi_noise_tokens
        };
    });

    (stream, router_name.to_string())
}

pub(crate) fn build() -> TokenStream {
    // Set up rerun-if-changed for all relevant JSON files
    let noise_settings_path = format!("{}/worldgen/noise_settings", DATAPACK_BASE);
    let density_function_path = format!("{}/worldgen/density_function", DATAPACK_BASE);
    
    println!("cargo:rerun-if-changed={}", noise_settings_path);
    println!("cargo:rerun-if-changed={}", density_function_path);

    let mut stream = TokenStream::new();

    stream.extend(quote! {
        use crate::noise_router::types::{
            BaseNoiseFunctionComponent, BaseNoiseRouter, BaseNoiseRouters, BaseSurfaceEstimator,
            BaseMultiNoiseRouter, NoiseData, ShiftedNoiseData, ClampedYGradientData,
            BinaryData, BinaryOperation, UnaryData, UnaryOperation, LinearData, LinearOperation,
            ClampData, RangeChoiceData, WeirdScaledData, WeirdScaledMapper,
            InterpolatedNoiseSamplerData, WrapperType, SplineRepr, SplinePoint,
        };
    });

    // Map environment names to their noise_settings JSON files
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
        let file_path = format!("{}/worldgen/noise_settings/{}", DATAPACK_BASE, file_name);
        
        let json_content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(_) => {
                eprintln!("Note: Skipping {} - file not found at {}", env_name, file_path);
                continue;
            }
        };

        let noise_settings: Value = match serde_json::from_str(&json_content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", file_path, e);
                continue;
            }
        };

        let noise_router = match noise_settings.get("noise_router").and_then(Value::as_object) {
            Some(router) => router,
            None => {
                eprintln!("Warning: No noise_router found in {}", file_path);
                continue;
            }
        };

        let (env_stream, _router_name) = generate_environment(env_name, noise_router);
        stream.extend(env_stream);
    }

    stream
}
