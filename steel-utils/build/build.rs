//! Build script for steel-utils that generates code from data files.

use std::{fs, path::Path, process::Command};

mod density_function_gen;
mod noise_params_gen;
use text_components::build::build_translations;

mod translations;

const FMT: bool = true;

const OUT_DIR: &str = "src/generated/vanilla_translations";
const IDS: &str = "ids";
const REGISTRY: &str = "registry";
const NOISE_PARAMS: &str = "noise_params";
const DENSITY_FUNCTIONS: &str = "density_functions";

/// Main build script entry point that generates code from data files.
pub fn main() {
    if !Path::new(OUT_DIR).exists() {
        fs::create_dir_all(OUT_DIR).expect("Failed to create output directory");
    }

    let content = build_translations("build_assets/en_us.json");
    fs::write(format!("{OUT_DIR}/{IDS}.rs"), content.to_string())
        .expect("Failed to write translations ids file");

    let content = translations::build();
    fs::write(format!("{OUT_DIR}/{REGISTRY}.rs"), content.to_string())
        .expect("Failed to write translations registry file");

    // Generate noise parameters
    let content = noise_params_gen::build();
    fs::write(format!("{OUT_DIR}/{NOISE_PARAMS}.rs"), content.to_string())
        .expect("Failed to write noise params file");

    // Generate density functions
    let content = density_function_gen::build();
    fs::write(
        format!("{OUT_DIR}/{DENSITY_FUNCTIONS}.rs"),
        content.to_string(),
    )
    .expect("Failed to write density functions file");

    if FMT && let Ok(entries) = fs::read_dir(OUT_DIR) {
        for entry in entries.flatten() {
            let _ = Command::new("rustfmt").arg(entry.path()).output();
        }
    }
}
