//! API-related functionality

pub mod client;
pub mod handlers;
pub mod models;
pub mod server;

// Re-export commonly used items
pub use models::ChatCompletionRequest; 