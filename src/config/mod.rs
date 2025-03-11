//! Configuration module

mod settings;
pub mod aisettings;

pub use settings::Config;

use dotenv::dotenv;

/// Load configuration from command line arguments and environment variables
pub fn load_from_args() -> Result<Config, String> {
    dotenv().ok();
    Ok(Config::from_args())
}

/// Load configuration from command line arguments and environment variables
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    dotenv().ok();
    Ok(Config::from_args())
} 