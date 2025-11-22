//! # Steel Core
//!
//! The core library for the Steel Minecraft server. Handles everything related to the PLAY state.
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::cargo,
    missing_docs,
    clippy::unwrap_used
)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]

use crate::chunk::chunk_map::ChunkMap;

pub mod chunk;
pub mod config;
pub mod player;
pub mod server;
pub mod world;
