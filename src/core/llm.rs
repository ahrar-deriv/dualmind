//! LLM interaction functionality

use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::io::Write;

use crate::config::Config;
use crate::models::{Message, Role};
use crate::streaming::process_stream;

/// Call reasoner model with context for reasoning
pub async fn call_reasoner_with_context(
    client: &Client,
    api_key: &str,
    session_messages: &[Message],
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Format the context for the reasoner
    // We need to limit context to avoid token limits
    let max_context_messages = 10;
    let _context_messages = if session_messages.len() > max_context_messages {
        &session_messages[session_messages.len() - max_context_messages..]
    } else {
        session_messages
    };

    // Check if this is a coding request
    let is_coding = is_coding_request(session_messages.last().map_or("", |m| &m.content));

    // Format messages for the API request with appropriate system prompt
    let system_content = if is_coding {
        "You are a reasoning engine that helps prepare structured thinking for a coding assistant. Think step by step about how to approach this request, considering:

1. Problem Analysis: Break down the user's request into clear components
2. Technical Considerations: Identify languages, frameworks, or specific technical requirements
3. Implementation Strategy: Outline a clear approach to solving the problem
4. Potential Challenges: Note any edge cases or difficulties that might arise
5. Code Structure: Suggest how the code should be organized

Your reasoning will be wrapped in <think></think> tags and will be used directly by a coding model to implement the solution."
    } else {
        "You are a reasoning engine that helps prepare structured thinking for an AI assistant. Think step by step about how to approach this request, considering:

1. Request Analysis: Break down the user's request into clear components
2. Relevant Knowledge: Identify key concepts, facts, or information needed to address the request
3. Response Strategy: Outline a clear approach to answering the question or addressing the request
4. Potential Nuances: Note any complexities, ambiguities, or important considerations
5. Response Structure: Suggest how to organize the information in a helpful way

Your reasoning will be wrapped in <think></think> tags and will be used directly by an assistant to formulate a response."
    };

    let mut api_messages = vec![json!({
        "role": "system",
        "content": system_content
    })];

    for message in session_messages {
        api_messages.push(json!({
            "role": message.role.to_string().to_lowercase(),
            "content": message.content
        }));
    }

    let request_body = json!({
        "model": config.reasoning_model,
        "messages": api_messages,
        "temperature": config.temperature,
        "stream": true
    });

    // Log the messages being sent to the reasoning model
    println!("\n=== MESSAGES SENT TO REASONING MODEL ===");
    println!("Model: {}", config.reasoning_model);
    for (i, message) in api_messages.iter().enumerate() {
        let role = message["role"].as_str().unwrap_or("unknown");
        let content = message["content"].as_str().unwrap_or("empty content");
        println!("\nMessage {}: Role = {}", i+1, role);
        println!("Content: {}", content);
        println!("-----------------------------------");
    }
    println!("========================================\n");

    // Add debug logging to check the API key
    println!("Debug - API Key: {}", if api_key.is_empty() { "EMPTY" } else { "NOT EMPTY" });
    println!("Debug - API URL: {}", config.api_url);

    let response = client
        .post(&format!("{}/v1/chat/completions", config.api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed: {} - {}", status, error_text).into());
    }

    let mut accumulated_response = String::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        for line in chunk_str.lines() {
            if let Some(content) = process_stream(line) {
                print!("{}", content);
                std::io::stdout().flush()?;
                accumulated_response.push_str(&content);
            }
        }
    }

    // Add think tags if not already present and ensure proper formatting
    let mut formatted_response = accumulated_response.trim().to_string();

    // Remove any existing think tags to avoid duplication
    formatted_response = formatted_response
        .replace("<think>", "")
        .replace("</think>", "");

    // Add the think tags with proper formatting
    formatted_response = format!("<think>\n{}\n</think>", formatted_response);

    accumulated_response = formatted_response;

    println!();
    Ok(accumulated_response)
}

/// Call crafter model with context for response generation
pub async fn call_crafter_with_context(
    client: &Client,
    api_key: &str,
    session_messages: &[Message],
    reasoning: &str,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Check if this is a coding request
    let is_coding = session_messages.iter()
        .filter(|m| m.role == Role::User)
        .last()
        .map(|m| is_coding_request(&m.content))
        .unwrap_or(false);

    // Create system and user content
    let system_content = if is_coding {
        format!("You are a coding assistant. Format your response in Markdown with proper code blocks. Use the following reasoning to help implement a solution: {}", reasoning)
    } else {
        format!("You are a helpful assistant. Format your response in Markdown. Use the following reasoning to help craft a response: {}", reasoning)
    };

    // Get the user content from the last user message
    let user_content = session_messages.iter()
        .filter(|m| m.role == Role::User)
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Check if we're using a Gemini model
    let is_gemini = config.craft_model.contains("gemini");
    
    // For Gemini models, we need to use a different approach
    if is_gemini {
        return call_gemini_model(client, api_key, session_messages, &system_content, config).await;
    }

    // Create the request body
    let request_body = json!({
        "model": config.craft_model,
        "messages": [
            {
                "role": "system",
                "content": system_content
            },
            {
                "role": "user",
                "content": user_content
            }
        ],
        "temperature": config.temperature,
        "stream": false
    });

    // Send the request
    let response = client
        .post(&format!("{}/v1/chat/completions", config.api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed: {} - {}", status, error_text).into());
    }

    let response_json: serde_json::Value = response.json().await?;
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}

/// Stream crafter model response
pub async fn stream_crafter_response(
    client: &Client,
    api_key: &str,
    session_messages: &[Message],
    reasoning: &str,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Check if this is a coding request
    let is_coding = session_messages.iter()
        .filter(|m| m.role == Role::User)
        .last()
        .map(|m| is_coding_request(&m.content))
        .unwrap_or(false);

    // Create a system message with the reasoning
    let system_message = if is_coding {
        format!("You are a coding assistant. Use the following reasoning to help implement a solution: {}", reasoning)
    } else {
        format!("You are a helpful assistant. Use the following reasoning to help craft a response: {}", reasoning)
    };

    // Create a vector of messages for the API call
    let mut api_messages = vec![
        json!({
            "role": "system",
            "content": system_message
        })
    ];

    // Add session messages
    for message in session_messages {
        api_messages.push(json!({
            "role": message.role.to_string().to_lowercase(),
            "content": message.content
        }));
    }

    // Determine the API provider
    let is_openrouter = crate::config::aisettings::is_openrouter(&config.api_url);
    let _is_litellm = crate::config::aisettings::is_litellm(&config.api_url);

    // Create the request body
    let mut request_body = json!({
        "model": config.craft_model,
        "messages": api_messages,
        "temperature": config.temperature,
        "stream": true
    });

    // Add provider-specific fields
    if is_openrouter {
        request_body["http_referer"] = json!("https://app.dualmind.ai");
        request_body["title"] = json!("DualMind API Client");
    }

    // Create the request
    let request = client
        .post(&format!("{}/v1/chat/completions", config.api_url))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body);

    // Send the request
    let response = request.send().await?;

    // Check if the request was successful
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed: {}", error_text).into());
    }

    // Process the streaming response
    let mut buffer = String::new();
    let mut response_body = response.bytes_stream();

    // Process each chunk as it arrives
    while let Some(chunk_result) = response_body.next().await {
        let chunk = chunk_result?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        
        // Split the chunk into lines
        for line in chunk_str.lines() {
            if let Some(content) = crate::streaming::process_stream(line) {
                print!("{}", content);
                std::io::stdout().flush()?;
                buffer.push_str(&content);
            }
        }
    }

    println!(); // Add a newline at the end
    Ok(buffer)
}

/// Call Gemini model with special handling
async fn call_gemini_model(
    client: &Client,
    api_key: &str,
    session_messages: &[Message],
    system_message: &str,
    config: &Config,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // For Gemini, we need to combine the system message with the first user message
    let mut combined_messages = Vec::new();
    
    // Find the first user message
    let mut found_user = false;
    for message in session_messages {
        if message.role == Role::User && !found_user {
            // Combine system message with first user message
            combined_messages.push(json!({
                "role": "user",
                "content": format!("{}\n\nUser request: {}", system_message, message.content)
            }));
            found_user = true;
        } else {
            // Add other messages as they are
            combined_messages.push(json!({
                "role": message.role.to_string().to_lowercase(),
                "content": message.content
            }));
        }
    }
    
    // If no user message was found, add the system message as a user message
    if !found_user {
        combined_messages.push(json!({
            "role": "user",
            "content": system_message
        }));
    }
    
    // Create the request body for Gemini
    let request_body = json!({
        "model": config.craft_model,
        "messages": combined_messages,
        "temperature": config.temperature
    });
    
    println!("Sending request to Gemini model...");
    
    // Create and send the request
    let response = client
        .post(&config.api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;
    
    // Check if the request was successful
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed: {}", error_text).into());
    }
    
    // Parse the response
    let response_json: serde_json::Value = response.json().await?;
    
    // Extract the content from the response
    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    
    // Print the response in chunks to simulate streaming
    for chunk in content.chars().collect::<Vec<char>>().chunks(5) {
        let chunk_str: String = chunk.iter().collect();
        print!("{}", chunk_str);
        std::io::stdout().flush()?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    Ok(content)
}

/// Process reasoner call and handle errors
pub async fn process_reasoner_call(client: &Client, config: &Config, user_content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Debug log to see what user_content contains
    println!("\n=== DEBUG: USER CONTENT RECEIVED ===");
    println!("Length: {}", user_content.len());
    println!("Content: '{}'", user_content);
    println!("===================================\n");
    
    // Create a message array with the user's content, ensuring it's not empty
    let user_message = if user_content.trim().is_empty() {
        "Hello, I need assistance.".to_string() // Default message if empty
    } else {
        user_content.to_string()
    };
    
    let messages = vec![
        Message {
            role: Role::User,
            content: user_message,
        }
    ];
    
    // Pass the messages to the reasoning model
    match call_reasoner_with_context(client, &config.api_key, &messages, config).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Reasoning model error: {}", e).into()),
    }
}

/// Clean up response text
pub fn clean_response_text(response: &str) -> String {
    // Remove any lines that look like thinking about the response
    // Remove any meta-commentary
    let mut cleaned = response.to_string();

    // Remove any lines that start with common meta-commentary patterns
    let patterns = vec![
        "Let me think about this",
        "I'll help you with",
        "I'll assist you with",
        "I'll provide",
        "Here's my response",
        "Let me respond to",
    ];

    for pattern in patterns {
        if let Some(idx) = cleaned.find(pattern) {
            if idx < 50 {
                // Only remove if it's near the beginning
                if let Some(newline_idx) = cleaned[idx..].find('\n') {
                    cleaned = cleaned[idx + newline_idx + 1..].to_string();
                }
            }
        }
    }

    cleaned.trim().to_string()
}

/// Check if a message is a coding request
pub fn is_coding_request(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    
    // Check for explicit code-related keywords
    let code_keywords = [
        "code", "function", "program", "script", "algorithm",
        "implement", "programming", "developer", "software",
        "class", "method", "variable", "compile", "debug",
        "syntax", "library", "framework", "api", "database",
        "sql", "html", "css", "javascript", "python", "java",
        "c++", "rust", "go", "typescript", "php", "ruby",
        "swift", "kotlin", "scala", "perl", "bash", "shell",
    ];
    
    for keyword in &code_keywords {
        if content_lower.contains(keyword) {
            return true;
        }
    }
    
    // Check for code block markers
    if content.contains("```") || content.contains("`") {
        return true;
    }
    
    // Check for common coding patterns
    if content.contains("def ") || content.contains("function ") || 
       content.contains("class ") || content.contains("import ") ||
       content.contains("from ") || content.contains("#include") ||
       content.contains("public static") || content.contains("fn ") {
        return true;
    }
    
    false
}

/// Check if a message is a continuation request
pub fn is_continuation_request(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    
    // Check for explicit continuation keywords
    let continuation_keywords = [
        "continue", "go on", "proceed", "keep going", "next",
        "more", "further", "expand", "elaborate", "additional",
        "furthermore", "moreover", "also", "besides", "additionally",
        "what else", "tell me more", "and then", "after that",
        "what next", "what follows", "next step", "then what",
    ];
    
    for keyword in &continuation_keywords {
        if content_lower.contains(keyword) {
            return true;
        }
    }
    
    // Check if the message is very short (likely a continuation prompt)
    if content.trim().split_whitespace().count() < 5 {
        return true;
    }
    
    false
} 