//! Core application logic

pub mod llm;
mod processor;
mod types;

pub use processor::process_data;
pub use types::{DataType, ProcessResult};

// Remove the run function since it's now in lib.rs 