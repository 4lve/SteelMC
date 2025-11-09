use crate::random::{
    PositionalRandom, Random, RandomSource, RandomSplitter, gaussian::MarsagliaPolarGaussian,
    get_seed,
};

// Xoroshiro128PlusPlusRandom
pub struct Xoroshiro {
    seed_lo: u64,
    seed_hi: u64,
    next_gaussian: Option<f64>,
}

pub struct XoroshiroSplitter {
    seed_lo: u64,
    seed_hi: u64,
}

// Ratios used in the mix functions
const GOLDEN_RATIO_64: u64 = 0x9E3779B97F4A7C15;
const SILVER_RATIO_64: u64 = 0x6A09E667F3BCC909;

impl Xoroshiro {
    pub fn from_seed(seed: u64) -> Self {
        // From RandomSupport
        let (lo, hi) = Self::upgrade_seed_to_128_bit(seed);
        let lo = mix_stafford_13(lo);
        let hi = mix_stafford_13(hi);
        Self::new(lo, hi)
    }

    pub fn from_seed_unmixed(seed: u64) -> Self {
        // From RandomSupport and
        let (lo, hi) = Self::upgrade_seed_to_128_bit(seed);
        Self::new(lo, hi)
    }

    fn new(lo: u64, hi: u64) -> Self {
        let (lo, hi) = if (lo | hi) == 0 {
            (GOLDEN_RATIO_64, SILVER_RATIO_64)
        } else {
            (lo, hi)
        };
        Self {
            seed_lo: lo,
            seed_hi: hi,
            next_gaussian: None,
        }
    }

    fn upgrade_seed_to_128_bit(seed: u64) -> (u64, u64) {
        let lo = seed ^ SILVER_RATIO_64;
        let hi = lo.wrapping_add(GOLDEN_RATIO_64);
        (lo, hi)
    }

    fn next(&mut self, bits: u64) -> u64 {
        self.next_random() >> (64 - bits)
    }

    fn next_random(&mut self) -> u64 {
        let l = self.seed_lo;
        let m = self.seed_hi;
        let n = l.wrapping_add(m).rotate_left(17).wrapping_add(l);
        let m = m ^ l;
        self.seed_lo = l.rotate_left(49) ^ m ^ (m << 21);
        self.seed_hi = m.rotate_left(28);
        n
    }
}

impl MarsagliaPolarGaussian for Xoroshiro {
    fn stored_next_gaussian(&self) -> Option<f64> {
        self.next_gaussian
    }

    fn set_stored_next_gaussian(&mut self, value: Option<f64>) {
        self.next_gaussian = value;
    }
}

fn mix_stafford_13(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

impl Random for Xoroshiro {
    fn fork(&mut self) -> Self {
        Self::new(self.next_random(), self.next_random())
    }

    fn next_i32(&mut self) -> i32 {
        self.next_random() as i32
    }

    fn next_i32_bounded(&mut self, bound: i32) -> i32 {
        let mut l = (self.next_i32() as u64) & 0xFFFFFFFF;
        let mut m = l.wrapping_mul(bound as u64);
        let mut n = m & 0xFFFFFFFF;
        if n < bound as u64 {
            let i = (((!bound).wrapping_add(1)) as u64) % (bound as u64);
            while n < i {
                l = (self.next_i32() as u64) & 0xFFFFFFFF;
                m = l.wrapping_mul(bound as u64);
                n = m & 0xFFFFFFFF;
            }
        }
        let o = m >> 32;
        o as i32
    }

    fn next_i64(&mut self) -> i64 {
        self.next_random() as i64
    }

    fn next_f32(&mut self) -> f32 {
        self.next(24) as f32 * 5.9604645e-8
    }

    fn next_f64(&mut self) -> f64 {
        self.next(53) as f64 * 1.110223e-16
    }

    fn next_bool(&mut self) -> bool {
        (self.next_random() & 1) != 0
    }

    fn next_gaussian(&mut self) -> f64 {
        self.calculate_gaussian()
    }

    fn next_positional(&mut self) -> RandomSplitter {
        RandomSplitter::Xoroshiro(XoroshiroSplitter {
            seed_lo: self.next_random(),
            seed_hi: self.next_random(),
        })
    }
}

impl PositionalRandom for XoroshiroSplitter {
    fn at(&self, x: i32, y: i32, z: i32) -> RandomSource {
        let l = get_seed(x, y, z) as u64;
        let m = l ^ self.seed_lo;

        RandomSource::Xoroshiro(Xoroshiro::new(m, self.seed_hi))
    }

    fn with_hash_of(&self, name: &str) -> RandomSource {
        let bytes = md5::compute(name.as_bytes());
        let l = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let m = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        RandomSource::Xoroshiro(Xoroshiro::new(l ^ self.seed_lo, m ^ self.seed_hi))
    }

    fn with_seed(&self, seed: u64) -> RandomSource {
        RandomSource::Xoroshiro(Xoroshiro::new(seed ^ self.seed_lo, seed ^ self.seed_hi))
    }
}
