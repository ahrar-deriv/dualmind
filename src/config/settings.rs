//! Configuration settings

use dotenv::dotenv;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub reasoning_model: String,
    pub craft_model: String,
    pub temperature: f32,
    pub api_url: String,
    pub api_key: String,
}

impl Config {
    pub fn new() -> Result<Self, String> {
        // First try to load from the current working directory
        if let Ok(_) = dotenv::from_filename("./.env") {
            println!("Loaded configuration from ./.env file");
        } else {
            // Then try to load from the executable directory
            if let Ok(_) = dotenv::dotenv() {
                println!("Loaded configuration from default .env file");
            } else {
                println!("No .env file found, using default or command-line configuration");
            }
        }

        // Default values
        let reasoning_model = env::var("REASONING_MODEL")
            .unwrap_or_else(|_| String::from("reasoning-model-name"));
        let craft_model =
            env::var("CRAFT_MODEL").unwrap_or_else(|_| String::from("crafting-model-name"));
        let temperature = env::var("TEMPERATURE")
            .ok()
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.7);
        let api_url =
            env::var("API_URL").unwrap_or_else(|_| String::from("http://localhost:1234"));
        let api_key = env::var("R_API_KEY").unwrap_or_else(|_| String::from(""));

        // Check if API key is set
        if api_key.is_empty() {
            return Err("R_API_KEY environment variable must be set in .env file or provided via command line".to_string());
        }

        Ok(Self {
            reasoning_model,
            craft_model,
            temperature,
            api_url,
            api_key,
        })
    }

    pub fn from_args() -> Self {
        // Load environment variables from .env file
        dotenv().ok();

        // Parse command line arguments
        let args: Vec<String> = env::args().collect();

        // Default values
        let mut reasoning_model = env::var("REASONING_MODEL")
            .unwrap_or_else(|_| String::from("reasoning-model-name"));
        let mut craft_model =
            env::var("CRAFT_MODEL").unwrap_or_else(|_| String::from("crafting-model-name"));
        let mut temperature = env::var("TEMPERATURE")
            .ok()
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.7);
        let mut api_url =
            env::var("API_URL").unwrap_or_else(|_| String::from("http://localhost:1234"));
        let mut api_key = env::var("R_API_KEY").unwrap_or_else(|_| String::from(""));

        // Process each argument (command line args override env vars)
        for arg in args.iter() {
            if let Some(model) = arg.strip_prefix("--reasoning_model=") {
                reasoning_model = model.to_string();
            } else if let Some(model) = arg.strip_prefix("--craft_model=") {
                craft_model = model.to_string();
            } else if let Some(temp) = arg.strip_prefix("--temperature=") {
                if let Ok(t) = temp.parse::<f32>() {
                    temperature = t;
                }
            } else if let Some(url) = arg.strip_prefix("--api_url=") {
                api_url = url.to_string();
            } else if let Some(key) = arg.strip_prefix("--api_key=") {
                api_key = key.to_string();
            }
        }

        Self {
            reasoning_model,
            craft_model,
            temperature,
            api_url,
            api_key,
        }
    }
} 