use serde::{Deserialize, Serialize};

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Content part for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { url: String },
    ImageBase64 { data: String, media_type: String },
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    #[serde(flatten)]
    content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Text { content: String },
    Parts { content: Vec<ContentPart> },
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text {
                content: content.into(),
            },
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text {
                content: content.into(),
            },
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text {
                content: content.into(),
            },
        }
    }

    pub fn user_with_parts(parts: Vec<ContentPart>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Parts { content: parts },
        }
    }

    pub fn content_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text { content } => Some(content),
            MessageContent::Parts { content } => {
                content.iter().find_map(|p| {
                    if let ContentPart::Text { text } = p {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
            }
        }
    }

    pub fn content_parts(&self) -> Vec<&ContentPart> {
        match &self.content {
            MessageContent::Text { .. } => {
                vec![]
            }
            MessageContent::Parts { content } => content.iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content_text(), Some("Hello"));
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::assistant("Hi there!");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("\"content\":\"Hi there!\""));
    }
}
