// Event types for JS plugin hook system
//
// This module defines the core Event enum that represents different
// lifecycle and runtime events in ZeroClaw that plugins can hook into.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum Event {
    MessageReceived {
        channel_id: String,
        channel_type: String,
        message: Value,
        session_id: Option<String>,
    },

    ToolCallPre {
        tool_name: String,
        input: Value,
        session_id: Option<String>,
    },

    ToolCallPost {
        tool_name: String,
        result: Value,
        session_id: Option<String>,
    },

    LlmRequest {
        provider: String,
        model: String,
        messages: Vec<Value>,
        options: Value,
    },

    SessionUpdate {
        session_id: String,
        context: Value,
    },

    BeforeAgentStart {
        config: Value,
    },

    Custom {
        namespace: String,
        name: String,
        payload: Value,
    },
}

impl Event {
    pub fn name(&self) -> Cow<str> {
        match self {
            Event::MessageReceived { .. } => Cow::Borrowed("message.received"),
            Event::ToolCallPre { .. } => Cow::Borrowed("tool.call.pre"),
            Event::ToolCallPost { .. } => Cow::Borrowed("tool.call.post"),
            Event::LlmRequest { .. } => Cow::Borrowed("llm.request"),
            Event::SessionUpdate { .. } => Cow::Borrowed("session.update"),
            Event::BeforeAgentStart { .. } => Cow::Borrowed("before.agent.start"),
            Event::Custom { name, .. } => Cow::Borrowed(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_name_message_received() {
        let event = Event::MessageReceived {
            channel_id: "123".to_string(),
            channel_type: "discord".to_string(),
            message: Value::Null,
            session_id: None,
        };
        assert_eq!(event.name().as_ref(), "message.received");
    }

    #[test]
    fn event_serialization() {
        let event = Event::MessageReceived {
            channel_id: "123".to_string(),
            channel_type: "discord".to_string(),
            message: serde_json::json!({"content": "hello"}),
            session_id: Some("session-abc".to_string()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "message.received");
    }
}
