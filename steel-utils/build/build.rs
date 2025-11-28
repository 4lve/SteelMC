//! Build script for steel-utils that generates translation constants.

use std::{fs, path::Path, process::Command};

mod translations;

const FMT: bool = true;

const OUT_DIR: &str = "src/generated";
const TRANSLATIONS: &str = "vanilla_translations";

/// Main build script entry point that generates translation constants.
pub fn main() {
    if !Path::new(OUT_DIR).exists() {
        fs::create_dir_all(OUT_DIR).expect("Failed to create output directory");
    }

    let content = translations::build();
    fs::write(format!("{OUT_DIR}/{TRANSLATIONS}.rs"), content.to_string())
        .expect("Failed to write translations file");

    if FMT && let Ok(entries) = fs::read_dir(OUT_DIR) {
        for entry in entries.flatten() {
            let _ = Command::new("rustfmt").arg(entry.path()).output();
        }
    }
}
