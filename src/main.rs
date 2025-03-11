//! Application entry point

use std::env;

fn main() {
    // Process command-line arguments
    let args: Vec<String> = env::args().collect();
    for arg in &args {
        if arg.starts_with("--api_key=") {
            unsafe {
                env::set_var("R_API_KEY", arg.trim_start_matches("--api_key="));
            }
        } else if arg.starts_with("--api_url=") {
            unsafe {
                env::set_var("API_URL", arg.trim_start_matches("--api_url="));
            }
        } else if arg.starts_with("--reasoning_model=") {
            unsafe {
                env::set_var("REASONING_MODEL", arg.trim_start_matches("--reasoning_model="));
            }
        } else if arg.starts_with("--coding_model=") {
            unsafe {
                env::set_var("CODING_MODEL", arg.trim_start_matches("--coding_model="));
            }
        } else if arg.starts_with("--temperature=") {
            unsafe {
                env::set_var("TEMPERATURE", arg.trim_start_matches("--temperature="));
            }
        }
    }

    // Run the application
    if let Err(e) = dualmind::run() {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}
