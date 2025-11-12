//! # Steel Protocol
//!
//! The core library for the Steel Minecraft server. Handles everything related to the PLAY state.
#![warn(clippy::all, clippy::pedantic, clippy::cargo, missing_docs)]
#![allow(
    clippy::single_call_fn,
    clippy::multiple_inherent_impl,
    clippy::shadow_unrelated,
    clippy::missing_errors_doc,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::cargo_common_metadata
)]
pub mod packet_reader;
pub mod packet_traits;
pub mod packet_writer;
pub mod packets;
pub mod utils;
