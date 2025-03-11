//! Streaming functionality

/// Process a stream line and extract content
pub fn process_stream(line: &str) -> Option<String> {
    // Skip empty lines and OpenRouter processing messages
    if line.is_empty() || line == "data: " || line == "[DONE]" || line.contains("OPENROUTER PROCESSING") {
        return None;
    }

    // Check if the line starts with "data: "
    let json_str = if line.starts_with("data: ") {
        // Skip the "data: " prefix
        &line[6..]
    } else {
        line
    };

    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Check for OpenAI format
        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                // Check for delta content
                if let Some(content) = first_choice
                    .get("delta")
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    if !content.is_empty() {
                        return Some(content.to_string());
                    }
                }
                
                // Check for message content
                if let Some(content) = first_choice
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    if !content.is_empty() {
                        return Some(content.to_string());
                    }
                }
            }
        }
        
        // Check for Anthropic format
        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }

    // If we couldn't parse it as JSON or find the content, return None
    None
} 