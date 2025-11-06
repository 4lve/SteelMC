use std::{fmt::Debug, hash::Hash};

use steel_utils::BlockStateId;

/// 3d array indexed by y,z,x
type Cube<T, const DIM: usize> = [[[T; DIM]; DIM]; DIM];

#[derive(Debug, Clone)]
pub struct HeterogeneousPalette<V: Hash + Eq + Copy, const DIM: usize> {
    cube: Box<Cube<V, DIM>>,
    // Keeps track of how many diffrent times each value appears in the cube. (value, count)
    palette: Vec<(V, u16)>,
}

impl<V: Hash + Eq + Copy, const DIM: usize> HeterogeneousPalette<V, DIM> {
    fn get(&self, x: usize, y: usize, z: usize) -> V {
        debug_assert!(x < DIM);
        debug_assert!(y < DIM);
        debug_assert!(z < DIM);

        self.cube[y][z][x]
    }

    fn set(&mut self, x: usize, y: usize, z: usize, value: V) -> V {
        debug_assert!(x < DIM);
        debug_assert!(y < DIM);
        debug_assert!(z < DIM);

        let old_value = self.cube[y][z][x];

        if let Some((_, count)) = self.palette.iter_mut().find(|(v, _)| *v == value) {
            *count += 1;
        } else {
            self.palette.push((value, 1));
        }

        if let Some((index, (_, count))) = self
            .palette
            .iter_mut()
            .enumerate()
            .find(|(_, (v, _))| *v == old_value)
        {
            *count -= 1;
            if *count == 0 {
                self.palette.swap_remove(index);
            }
        }

        self.cube[y][z][x] = value;

        old_value
    }
}

#[derive(Debug, Clone)]
pub enum PalettedContainer<V: Hash + Eq + Copy + Default, const DIM: usize> {
    Homogeneous(V),
    Heterogeneous(HeterogeneousPalette<V, DIM>),
}

impl<V: Hash + Eq + Copy + Default + Debug, const DIM: usize> PalettedContainer<V, DIM> {
    pub const SIZE: usize = DIM;
    pub const VOLUME: usize = DIM * DIM * DIM;

    fn from_cube(cube: Box<Cube<V, DIM>>) -> Self {
        let mut palette: Vec<(V, u16)> = Vec::new();
        cube.iter().flatten().flatten().for_each(|v| {
            if let Some((_, count)) = palette.iter_mut().find(|(value, _)| value == v) {
                *count += 1;
            } else {
                palette.push((*v, 1));
            }
        });

        if palette.len() == 1 {
            Self::Homogeneous(palette[0].0)
        } else {
            Self::Heterogeneous(HeterogeneousPalette { cube, palette })
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> V {
        match self {
            Self::Homogeneous(value) => *value,
            Self::Heterogeneous(data) => data.get(x, y, z),
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: V) -> V {
        debug_assert!(x < Self::SIZE);
        debug_assert!(y < Self::SIZE);
        debug_assert!(z < Self::SIZE);

        match self {
            Self::Homogeneous(original) => {
                let original = *original;
                if value != original {
                    let mut cube = Box::new([[[original; DIM]; DIM]; DIM]);
                    cube[y][z][x] = value;
                    *self = Self::from_cube(cube);
                }
                original
            }
            Self::Heterogeneous(data) => {
                let original = data.set(x, y, z, value);
                if data.palette.len() == 1 {
                    *self = Self::Homogeneous(data.palette[0].0);
                }
                original
            }
        }
    }
}

pub type BlockPalette = PalettedContainer<BlockStateId, 16>;
