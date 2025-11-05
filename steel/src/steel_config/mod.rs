use base64::{Engine, prelude::BASE64_STANDARD};
use serde::Deserialize;
use std::{fs, net::SocketAddr, path::Path};
use steel_protocol::packet_traits::CompressionInfo;

const DEFAULT_CONFIG_STR: &str = include_str!("default_config.json5");
const DEFAULT_FAVICON_STR: &[u8] = include_bytes!("default_favicon.png");

#[derive(Debug, Clone, Deserialize)]
pub struct SteelConfig {
    pub server_address: SocketAddr,
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

impl SteelConfig {
    #[must_use]
    /// # Panics
    /// This function will panic if the config file does not exist and the directory cannot be created, or if the config file cannot be read or written.
    pub fn load_or_create(path: &Path) -> Self {
        let config = if path.exists() {
            let config_str = fs::read_to_string(path).unwrap();
            let config: SteelConfig = serde_json5::from_str(config_str.as_str()).unwrap();
            config.validate().unwrap();
            config
        } else {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, DEFAULT_CONFIG_STR).unwrap();
            let config: SteelConfig = serde_json5::from_str(DEFAULT_CONFIG_STR).unwrap();
            config.validate().unwrap();
            config
        };

        // If icon file doesnt exist, write it
        if config.use_favicon && !Path::new(&config.favicon).exists() {
            fs::write(Path::new(&config.favicon), DEFAULT_FAVICON_STR).unwrap();
        }

        config
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.view_distance == 0 {
            return Err("View distance must be greater than 0".to_string());
        }
        if self.view_distance > 32 {
            return Err("View distance must be less than or equal to 32".to_string());
        }
        if self.simulation_distance == 0 {
            return Err("Simulation distance must be greater than 0".to_string());
        }
        if self.simulation_distance > 32 {
            return Err("Simulation distance must be less than or equal to 32".to_string());
        }
        if let Some(compression) = self.compression {
            if compression.threshold.get() < 256 {
                return Err(
                    "Compression threshold must be greater than or equal to 256".to_string()
                );
            }
            if compression.level < 1 || compression.level > 9 {
                return Err("Compression level must be between 1 and 9".to_string());
            }
        }
        Ok(())
    }

    const PREFIX: &str = "data:image/png;base64,";

    #[must_use]
    pub fn load_favicon(&self) -> Option<String> {
        if self.use_favicon {
            let path = Path::new(&self.favicon);
            if path.exists() {
                let base64 = BASE64_STANDARD
                    .encode(fs::read(path).unwrap_or_else(|_| DEFAULT_FAVICON_STR.to_vec()));
                return Some(format!("{}{}", Self::PREFIX, base64));
            }
        }
        None
    }
}
