// Type definitions for the core module

#[derive(Debug)]
pub struct DataType {
    pub id: String,
    pub value: String,
}

#[derive(Debug)]
pub struct ProcessResult {
    pub success: bool,
    pub message: String,
} 