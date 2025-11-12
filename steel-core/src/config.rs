use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;
use std::{fs, path::Path, sync::LazyLock};
use steel_protocol::packet_traits::CompressionInfo;

#[cfg(feature = "stand-alone")]
const DEFAULT_FAVICON: &[u8] = include_bytes!("../../package-content/favicon.png");
const ICON_PREFIX: &str = "data:image/png;base64,";

const DEFAULT_CONFIG: &str = include_str!("../../package-content/steel_config.json5");

pub static STEEL_CONFIG: LazyLock<ServerConfig> =
    LazyLock::new(|| ServerConfig::load_or_create(Path::new("config/steel_config.json5")));

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server_port: u16,
    pub seed: String,
    pub max_players: u32,
    pub view_distance: u8,
    pub simulation_distance: u8,
    pub online_mode: bool,
    pub encryption: bool,
    pub motd: String,
    pub use_favicon: bool,
    pub favicon: String,
    pub enforce_secure_chat: bool,
    pub compression: Option<CompressionInfo>,
}

impl ServerConfig {
    #[must_use]
    /// # Panics
    /// This function will panic if the config file does not exist and the directory cannot be created, or if the config file cannot be read or written.
    pub fn load_or_create(path: &Path) -> Self {
        let config = if path.exists() {
            let config_str = fs::read_to_string(path).unwrap();
            let config: ServerConfig = serde_json5::from_str(config_str.as_str()).unwrap();
            config.validate().unwrap();
            config
        } else {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, DEFAULT_CONFIG).unwrap();
            let config: ServerConfig = serde_json5::from_str(DEFAULT_CONFIG).unwrap();
            config.validate().unwrap();
            config
        };

        // If icon file doesnt exist, write it
        #[cfg(feature = "stand-alone")]
        if config.use_favicon && !Path::new(&config.favicon).exists() {
            fs::write(Path::new(&config.favicon), DEFAULT_FAVICON).unwrap();
        }

        config
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=64).contains(&self.view_distance) {
            return Err("View distance must in range 1..64");
        }
        if !(1..=32).contains(&self.simulation_distance) {
            return Err("Simulation distance must in range 1..32");
        }
        if let Some(compression) = self.compression {
            if compression.threshold.get() < 256 {
                return Err("Compression threshold must be greater than or equal to 256");
            }
            if !(1..=9).contains(&compression.level) {
                return Err("Compression level must be between 1 and 9");
            }
        }
        Ok(())
    }

    /// Assemble the icon with only one alloc :)
    #[must_use]
    pub fn load_favicon(&self) -> Option<String> {
        if self.use_favicon {
            let path = Path::new(&self.favicon);
            if path.exists() {
                let icon = fs::read(path);

                if let Ok(icon) = icon {
                    let cap = ICON_PREFIX.len() + icon.len().div_ceil(3) * 4;
                    let mut base64 = String::with_capacity(cap);

                    base64 += ICON_PREFIX;
                    BASE64_STANDARD.encode_string(icon, &mut base64);

                    return Some(base64);
                } else {
                    #[cfg(feature = "stand-alone")]
                    {
                        let cap = ICON_PREFIX.len() + DEFAULT_FAVICON.len().div_ceil(3) * 4;
                        let mut base64 = String::with_capacity(cap);

                        base64 += ICON_PREFIX;
                        BASE64_STANDARD.encode_string(DEFAULT_FAVICON, &mut base64);

                        return Some(base64);
                    }
                    #[cfg(not(feature = "stand-alone"))]
                    return None;
                }
            }
        }
        None
    }
}
