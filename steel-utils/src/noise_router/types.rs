pub struct NoiseData {
    pub noise_id: &'static str,
    pub xz_scale: f64,
    pub y_scale: f64,
}
pub struct ShiftedNoiseData {
    pub xz_scale: f64,
    pub y_scale: f64,
    pub noise_id: &'static str,
}
#[derive(Copy, Clone)]
pub enum WeirdScaledMapper {
    Caves,
    Tunnels,
}
impl WeirdScaledMapper {
    #[inline]
    #[must_use]
    pub fn max_multiplier(&self) -> f64 {
        match self {
            Self::Tunnels => 2.0,
            Self::Caves => 3.0,
        }
    }
    #[inline]
    #[must_use]
    pub fn scale(&self, value: f64) -> f64 {
        match self {
            Self::Tunnels => {
                if value < -0.5 {
                    0.75
                } else if value < 0.0 {
                    1.0
                } else if value < 0.5 {
                    1.5
                } else {
                    2.0
                }
            }
            Self::Caves => {
                if value < -0.75 {
                    0.5
                } else if value < -0.5 {
                    0.75
                } else if value < 0.5 {
                    1.0
                } else if value < 0.75 {
                    2.0
                } else {
                    3.0
                }
            }
        }
    }
}
pub struct WeirdScaledData {
    pub noise_id: &'static str,
    pub mapper: WeirdScaledMapper,
}
pub struct InterpolatedNoiseSamplerData {
    pub scaled_xz_scale: f64,
    pub scaled_y_scale: f64,
    pub xz_factor: f64,
    pub y_factor: f64,
    pub smear_scale_multiplier: f64,
}
pub struct ClampedYGradientData {
    pub from_y: f64,
    pub to_y: f64,
    pub from_value: f64,
    pub to_value: f64,
}
#[derive(Copy, Clone)]
pub enum BinaryOperation {
    Add,
    Mul,
    Min,
    Max,
}
pub struct BinaryData {
    pub operation: BinaryOperation,
}
#[derive(Copy, Clone)]
pub enum LinearOperation {
    Add,
    Mul,
}
pub struct LinearData {
    pub operation: LinearOperation,
    pub argument: f64,
}
impl LinearData {
    #[inline]
    #[must_use]
    pub fn apply_density(&self, density: f64) -> f64 {
        match self.operation {
            LinearOperation::Add => density + self.argument,
            LinearOperation::Mul => density * self.argument,
        }
    }
}
#[derive(Copy, Clone)]
pub enum UnaryOperation {
    Abs,
    Square,
    Cube,
    HalfNegative,
    QuarterNegative,
    Squeeze,
}
pub struct UnaryData {
    pub operation: UnaryOperation,
}
impl UnaryData {
    #[inline]
    #[must_use]
    pub fn apply_density(&self, density: f64) -> f64 {
        match self.operation {
            UnaryOperation::Abs => density.abs(),
            UnaryOperation::Square => density * density,
            UnaryOperation::Cube => density * density * density,
            UnaryOperation::HalfNegative => {
                if density > 0.0 {
                    density
                } else {
                    density * 0.5
                }
            }
            UnaryOperation::QuarterNegative => {
                if density > 0.0 {
                    density
                } else {
                    density * 0.25
                }
            }
            UnaryOperation::Squeeze => {
                let clamped = density.clamp(-1.0, 1.0);
                clamped / 2.0 - clamped * clamped * clamped / 24.0
            }
        }
    }
}
pub struct ClampData {
    pub min_value: f64,
    pub max_value: f64,
}
impl ClampData {
    #[inline]
    #[must_use]
    pub fn apply_density(&self, density: f64) -> f64 {
        density.clamp(self.min_value, self.max_value)
    }
}
pub struct RangeChoiceData {
    pub min_inclusive: f64,
    pub max_exclusive: f64,
}
pub struct SplinePoint {
    pub location: f32,
    pub value: &'static SplineRepr,
    pub derivative: f32,
}
pub enum SplineRepr {
    Standard {
        location_function_index: usize,
        points: &'static [SplinePoint],
    },
    Fixed {
        value: f32,
    },
}
#[derive(Copy, Clone)]
pub enum WrapperType {
    Interpolated,
    CacheFlat,
    Cache2D,
    CacheOnce,
    CellCache,
}
pub enum BaseNoiseFunctionComponent {
    Beardifier,
    BlendAlpha,
    BlendOffset,
    BlendDensity {
        input_index: usize,
    },
    EndIslands,
    Noise {
        data: &'static NoiseData,
    },
    ShiftA {
        noise_id: &'static str,
    },
    ShiftB {
        noise_id: &'static str,
    },
    ShiftedNoise {
        shift_x_index: usize,
        shift_y_index: usize,
        shift_z_index: usize,
        data: &'static ShiftedNoiseData,
    },
    InterpolatedNoiseSampler {
        data: &'static InterpolatedNoiseSamplerData,
    },
    WeirdScaled {
        input_index: usize,
        data: &'static WeirdScaledData,
    },
    Wrapper {
        input_index: usize,
        wrapper: WrapperType,
    },
    Constant {
        value: f64,
    },
    ClampedYGradient {
        data: &'static ClampedYGradientData,
    },
    Binary {
        argument1_index: usize,
        argument2_index: usize,
        data: &'static BinaryData,
    },
    Linear {
        input_index: usize,
        data: &'static LinearData,
    },
    Unary {
        input_index: usize,
        data: &'static UnaryData,
    },
    Clamp {
        input_index: usize,
        data: &'static ClampData,
    },
    RangeChoice {
        input_index: usize,
        when_in_range_index: usize,
        when_out_range_index: usize,
        data: &'static RangeChoiceData,
    },
    Spline {
        spline: &'static SplineRepr,
    },
}
pub struct BaseNoiseRouter {
    pub full_component_stack: &'static [BaseNoiseFunctionComponent],
    pub barrier_noise: usize,
    pub fluid_level_floodedness_noise: usize,
    pub fluid_level_spread_noise: usize,
    pub lava_noise: usize,
    pub erosion: usize,
    pub depth: usize,
    pub final_density: usize,
    pub vein_toggle: usize,
    pub vein_ridged: usize,
    pub vein_gap: usize,
}
pub struct BaseSurfaceEstimator {
    pub full_component_stack: &'static [BaseNoiseFunctionComponent],
}
pub struct BaseMultiNoiseRouter {
    pub full_component_stack: &'static [BaseNoiseFunctionComponent],
    pub temperature: usize,
    pub vegetation: usize,
    pub continents: usize,
    pub erosion: usize,
    pub depth: usize,
    pub ridges: usize,
}
pub struct BaseNoiseRouters {
    pub noise: BaseNoiseRouter,
    pub surface_estimator: BaseSurfaceEstimator,
    pub multi_noise: BaseMultiNoiseRouter,
}
