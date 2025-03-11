//! Domain models

use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    #[serde(deserialize_with = "deserialize_message_content")]
    pub content: String,
}

fn deserialize_message_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    if let Some(s) = value.as_str() {
         Ok(s.to_string())
    } else if let Some(arr) = value.as_array() {
         let mut parts = Vec::new();
         for elem in arr {
             if let Some(s) = elem.as_str() {
                 parts.push(s.to_string());
             } else if let Some(obj) = elem.as_object() {
                 if let Some(text_val) = obj.get("text") {
                     if let Some(text_str) = text_val.as_str() {
                         parts.push(text_str.to_string());
                     } else {
                         return Err(de::Error::custom("Expected 'text' field as string in object"));
                     }
                 } else {
                     return Err(de::Error::custom("Expected object to contain 'text' field"));
                 }
             } else {
                 return Err(de::Error::custom("Unexpected element type in content array"));
             }
         }
         Ok(parts.join(" "))
    } else {
         Err(de::Error::custom("Expected string or array for message content"))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
} 