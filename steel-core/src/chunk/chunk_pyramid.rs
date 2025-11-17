//! This module contains the `ChunkPyramid`, which is used to check chunk dependencies.

use crate::chunk::chunk_access::ChunkStatus;
use crate::chunk::chunk_status_tasks::ChunkStatusTasks;
use std::cmp::max;
use std::sync::{Arc, LazyLock};

/// A map of radius to the required chunk status.
#[derive(Debug, Clone)]
pub struct ChunkDependencies {
    dependency_by_radius: Box<[ChunkStatus]>,
    radius_by_dependency: Box<[usize]>,
}

impl ChunkDependencies {
    /// Creates a new chunk dependencies.
    #[must_use]
    pub fn new(dependency_by_radius: Vec<ChunkStatus>) -> Self {
        let dependency_by_radius = dependency_by_radius.into_boxed_slice();
        let size = dependency_by_radius
            .first()
            .map_or(0, |s| s.get_index() + 1);
        let mut radius_by_dependency = vec![0; size].into_boxed_slice();

        for (radius, dependency) in dependency_by_radius.iter().enumerate() {
            let index = dependency.get_index();
            for i in 0..=index {
                if i < radius_by_dependency.len() {
                    radius_by_dependency[i] = radius;
                }
            }
        }

        Self {
            dependency_by_radius,
            radius_by_dependency,
        }
    }

    /// Gets the radius of the dependencies for the given status.
    #[must_use]
    pub fn get_radius_of(&self, status: ChunkStatus) -> usize {
        let index = status.get_index();
        if index >= self.radius_by_dependency.len() {
            // This would be an error in Java, but Rust prefers safe access.
            // Returning the max radius is a possible fallback.
            return self.get_radius();
        }
        self.radius_by_dependency[index]
    }

    /// Gets the radius of the dependencies.
    #[must_use]
    pub fn get_radius(&self) -> usize {
        self.dependency_by_radius.len().saturating_sub(1)
    }

    /// Gets the dependencies for the given distance.
    #[must_use]
    pub fn get(&self, distance: usize) -> Option<ChunkStatus> {
        self.dependency_by_radius.get(distance).copied()
    }
}

/// A task that generates a chunk.
pub type ChunkStatusTask = fn(/* ... world gen context args ... */);

/// A chunk step.
#[derive(Clone)]
pub struct ChunkStep {
    /// The target status of the step.
    pub target_status: ChunkStatus,
    /// The direct dependencies of the step.
    pub direct_dependencies: Arc<ChunkDependencies>,
    /// The accumulated dependencies of the step.
    pub accumulated_dependencies: Arc<ChunkDependencies>,
    /// The block state write radius of the step.
    pub block_state_write_radius: i32,
    /// The task of the step.
    pub task: ChunkStatusTask,
}

impl ChunkStep {
    /// Creates a new chunk step builder.
    #[must_use]
    pub fn builder(status: ChunkStatus, parent: Option<Arc<ChunkStep>>) -> Builder {
        Builder::new(status, parent)
    }
}

/// A builder for a chunk step.
pub struct Builder {
    status: ChunkStatus,
    parent: Option<Arc<ChunkStep>>,
    direct_dependencies_by_radius: Vec<ChunkStatus>,
    block_state_write_radius: i32,
    task: ChunkStatusTask,
}

impl Builder {
    #[must_use]
    /// Creates a new chunk step builder.
    ///
    /// # Panics
    ///
    /// Panics if the status is not the next status of the parent.
    pub fn new(status: ChunkStatus, parent: Option<Arc<ChunkStep>>) -> Self {
        let direct_dependencies_by_radius = if let Some(p) = &parent {
            assert!(
                p.target_status.next() == Some(status),
                "Out of order status: {:?}, expected next of {:?}",
                status,
                p.target_status
            );
            vec![p.target_status]
        } else {
            assert!(
                status.parent().is_none(),
                "Not starting with the first status: {status:?}"
            );
            Vec::new()
        };

        Self {
            status,
            parent,
            direct_dependencies_by_radius,
            block_state_write_radius: -1,
            task: || {},
        }
    }

    #[must_use]
    /// Adds a requirement to the step.
    ///
    /// # Panics
    ///
    /// Panics if the status is greater than the current status.
    pub fn add_requirement(mut self, status: ChunkStatus, radius: usize) -> Self {
        assert!(
            status < self.status,
            "Status {:?} can not be required by {:?}",
            status,
            self.status
        );

        let new_len = radius + 1;
        let old_len = self.direct_dependencies_by_radius.len();

        if new_len > old_len {
            self.direct_dependencies_by_radius.resize(new_len, status);
        }

        for i in 0..old_len.min(new_len) {
            self.direct_dependencies_by_radius[i] =
                self.direct_dependencies_by_radius[i].max(status);
        }
        self
    }

    #[must_use]
    /// Sets the block state write radius for the step.
    pub fn block_state_write_radius(mut self, radius: i32) -> Self {
        self.block_state_write_radius = radius;
        self
    }

    /// Sets the task for the step.
    #[must_use]
    pub fn set_task(mut self, task: ChunkStatusTask) -> Self {
        self.task = task;
        self
    }

    #[must_use]
    /// Builds the chunk step.
    pub fn build(self) -> ChunkStep {
        let accumulated_dependencies = self.build_accumulated_dependencies();
        let direct_dependencies = ChunkDependencies::new(self.direct_dependencies_by_radius);

        ChunkStep {
            target_status: self.status,
            direct_dependencies: Arc::new(direct_dependencies),
            accumulated_dependencies: Arc::new(accumulated_dependencies),
            block_state_write_radius: self.block_state_write_radius,
            task: self.task,
        }
    }

    fn build_accumulated_dependencies(&self) -> ChunkDependencies {
        if self.parent.is_none() {
            return ChunkDependencies::new(self.direct_dependencies_by_radius.clone());
        }
        let parent = self.parent.as_ref().expect("Parent is required");

        let radius_of_parent = self
            .direct_dependencies_by_radius
            .iter()
            .rposition(|&s| s >= parent.target_status)
            .unwrap_or(0);

        let parent_deps = &parent.accumulated_dependencies;
        let new_len = max(
            radius_of_parent + parent_deps.dependency_by_radius.len(),
            self.direct_dependencies_by_radius.len(),
        );
        let mut accumulated = Vec::with_capacity(new_len);

        for dist in 0..new_len {
            let dist_in_parent = dist.saturating_sub(radius_of_parent);

            let parent_dep = parent_deps.get(dist_in_parent);
            let direct_dep = self.direct_dependencies_by_radius.get(dist).copied();

            let dep = match (direct_dep, parent_dep) {
                (Some(d), Some(p)) => d.max(p),
                (Some(d), None) => d,
                (None, Some(p)) => p,
                (None, None) => continue,
            };
            accumulated.push(dep);
        }

        ChunkDependencies::new(accumulated)
    }
}

/// Represents the hierarchy and dependencies for chunk generation or loading.
pub struct ChunkPyramid {
    steps: Box<[Arc<ChunkStep>]>,
}

impl ChunkPyramid {
    /// Gets the step for the given status.
    #[must_use]
    pub fn get_step_to(&self, status: ChunkStatus) -> Arc<ChunkStep> {
        self.steps[status.get_index()].clone()
    }
}

/// A builder for a chunk pyramid.
pub struct ChunkPyramidBuilder {
    steps: Vec<Arc<ChunkStep>>,
}

impl Default for ChunkPyramidBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkPyramidBuilder {
    #[must_use]
    /// Creates a new chunk pyramid builder.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Adds a new step to the chunk pyramid.
    #[must_use]
    pub fn step<F>(mut self, status: ChunkStatus, op: F) -> Self
    where
        F: FnOnce(Builder) -> Builder,
    {
        let parent = self.steps.last().cloned();
        let step_builder = ChunkStep::builder(status, parent);
        let built_step = op(step_builder).build();
        self.steps.push(Arc::new(built_step));
        self
    }

    #[must_use]
    /// Builds the chunk pyramid.
    pub fn build(self) -> ChunkPyramid {
        ChunkPyramid {
            steps: self.steps.into_boxed_slice(),
        }
    }
}

/// The generation pyramid.
pub static GENERATION_PYRAMID: LazyLock<ChunkPyramid> = LazyLock::new(|| {
    ChunkPyramidBuilder::new()
        .step(ChunkStatus::Empty, |s| s)
        .step(ChunkStatus::StructureStarts, |s| {
            s.set_task(ChunkStatusTasks::generate_structure_starts)
        })
        .step(ChunkStatus::StructureReferences, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .set_task(ChunkStatusTasks::generate_structure_references)
        })
        .step(ChunkStatus::Biomes, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .set_task(ChunkStatusTasks::generate_biomes)
        })
        .step(ChunkStatus::Noise, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .add_requirement(ChunkStatus::Biomes, 1)
                .block_state_write_radius(0)
                .set_task(ChunkStatusTasks::generate_noise)
        })
        .step(ChunkStatus::Surface, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .add_requirement(ChunkStatus::Biomes, 1)
                .block_state_write_radius(0)
                .set_task(ChunkStatusTasks::generate_surface)
        })
        .step(ChunkStatus::Carvers, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .block_state_write_radius(0)
                .set_task(ChunkStatusTasks::generate_carvers)
        })
        .step(ChunkStatus::Features, |s| {
            s.add_requirement(ChunkStatus::StructureStarts, 8)
                .add_requirement(ChunkStatus::Carvers, 1)
                .block_state_write_radius(1)
                .set_task(ChunkStatusTasks::generate_features)
        })
        .step(ChunkStatus::InitializeLight, |s| {
            s.set_task(ChunkStatusTasks::initialize_light)
        })
        .step(ChunkStatus::Light, |s| {
            s.add_requirement(ChunkStatus::InitializeLight, 1)
                .set_task(ChunkStatusTasks::light)
        })
        .step(ChunkStatus::Spawn, |s| {
            s.add_requirement(ChunkStatus::Biomes, 1)
                .set_task(ChunkStatusTasks::generate_spawn)
        })
        .step(ChunkStatus::Full, |s| s.set_task(ChunkStatusTasks::full))
        .build()
});

/// The loading pyramid.
pub static LOADING_PYRAMID: LazyLock<ChunkPyramid> = LazyLock::new(|| {
    ChunkPyramidBuilder::new()
        .step(ChunkStatus::Empty, |s| s)
        .step(ChunkStatus::StructureStarts, |s| {
            s.set_task(ChunkStatusTasks::load_structure_starts)
        })
        .step(ChunkStatus::StructureReferences, |s| s)
        .step(ChunkStatus::Biomes, |s| s)
        .step(ChunkStatus::Noise, |s| s)
        .step(ChunkStatus::Surface, |s| s)
        .step(ChunkStatus::Carvers, |s| s)
        .step(ChunkStatus::Features, |s| s)
        .step(ChunkStatus::InitializeLight, |s| {
            s.set_task(ChunkStatusTasks::initialize_light)
        })
        .step(ChunkStatus::Light, |s| {
            s.add_requirement(ChunkStatus::InitializeLight, 1)
                .set_task(ChunkStatusTasks::light)
        })
        .step(ChunkStatus::Spawn, |s| s)
        .step(ChunkStatus::Full, |s| s.set_task(ChunkStatusTasks::full))
        .build()
});
