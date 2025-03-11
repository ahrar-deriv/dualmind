use axum::{body::Body, extract::Request, http::HeaderMap, middleware::Next, response::Response};
use serde_json::Value;
use std::time::Instant;

/// Middleware for logging API requests
pub async fn log_request(req: Request, next: Next) -> Response {
    let start_time = Instant::now();

    // Extract request details
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    // Clone the request body for logging if it's a POST/PUT request
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();
    let body_str = String::from_utf8_lossy(&bytes);

    // Only log body for POST/PUT requests
    let body_json = if method == "POST" || method == "PUT" {
        if let Ok(json) = serde_json::from_str::<Value>(&body_str) {
            // Redact sensitive information
            let mut json = json.clone();
            if let Some(obj) = json.as_object_mut() {
                if obj.contains_key("api_key") {
                    obj.insert("api_key".to_string(), Value::String("*****".to_string()));
                }
            }
            Some(json)
        } else {
            None
        }
    } else {
        None
    };

    // Reconstruct the request
    let body = Body::from(bytes);
    let req = Request::from_parts(parts, body);

    // Process the request
    let response = next.run(req).await;

    // Calculate duration
    let duration = start_time.elapsed();

    // Log the request details
    println!("\n=== API Request ===");
    println!(
        "Timestamp: {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    println!("Method: {}", method);
    println!("Path: {}", uri.path());
    println!("Duration: {:.2}ms", duration.as_secs_f64() * 1000.0);
    println!("Status: {}", response.status());

    // Log headers (excluding sensitive ones)
    println!("\nHeaders:");
    log_headers(&headers);

    // Log body if present
    if let Some(json) = body_json {
        println!("\nRequest Body:");
        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    }

    println!("================\n");

    response
}

/// Helper function to log headers while excluding sensitive information
fn log_headers(headers: &HeaderMap) {
    let sensitive_headers = vec!["authorization", "cookie", "x-api-key"];

    for (key, value) in headers.iter() {
        let header_name = key.as_str().to_lowercase();
        if sensitive_headers.contains(&header_name.as_str()) {
            println!("  {}: *****", key);
        } else {
            println!("  {}: {}", key, value.to_str().unwrap_or("[invalid]"));
        }
    }
}
