// Channels API bridge - connects JS plugins to ZeroClaw's Channel backends

use crate::channels::traits::{Channel, SendMessage};
use crate::js::error::JsPluginError;
use std::sync::Arc;

/// Bridge between JS plugins and ZeroClaw's messaging channels
///
/// This bridge provides controlled access to messaging channels for JS plugins.
/// Only explicitly allowed channels are accessible to maintain security boundaries.
pub struct JsChannelsBridge {
    /// Allowed channels accessible to plugins
    /// Stored as (name, channel) tuples for name-based lookup
    allowed_channels: Vec<(String, Arc<dyn Channel>)>,
}

impl JsChannelsBridge {
    /// Create a new channels bridge with an allowlist
    ///
    /// # Arguments
    ///
    /// * `allowed_channels` - List of (name, channel) tuples accessible to plugins
    pub fn new(allowed_channels: Vec<(String, Arc<dyn Channel>)>) -> Self {
        Self { allowed_channels }
    }

    /// Create a bridge with no channel access (deny-by-default)
    pub fn blocked() -> Self {
        Self {
            allowed_channels: vec![],
        }
    }

    /// Get a channel by name
    ///
    /// # Arguments
    ///
    /// * `name` - The channel name to look up
    ///
    /// # Returns
    ///
    /// Returns the channel if found and allowed
    pub fn get_channel(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.allowed_channels
            .iter()
            .find(|(channel_name, _)| channel_name == name)
            .map(|(_, channel)| Arc::clone(channel))
    }

    /// Send a message through a named channel
    ///
    /// # Arguments
    ///
    /// * `channel_name` - The name of the channel to send through
    /// * `content` - The message content
    /// * `recipient` - The recipient identifier
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the message was sent successfully
    pub async fn send_message(
        &self,
        channel_name: &str,
        content: &str,
        recipient: &str,
    ) -> Result<(), JsPluginError> {
        let channel = self
            .get_channel(channel_name)
            .ok_or_else(|| JsPluginError::Channel(format!("Channel not found: {}", channel_name)))?;

        let message = SendMessage::new(content, recipient);
        channel
            .send(&message)
            .await
            .map_err(|e| JsPluginError::Channel(format!("Send failed: {}", e)))
    }

    /// Send a message with subject through a named channel
    ///
    /// # Arguments
    ///
    /// * `channel_name` - The name of the channel to send through
    /// * `content` - The message content
    /// * `recipient` - The recipient identifier
    /// * `subject` - Optional subject line
    pub async fn send_message_with_subject(
        &self,
        channel_name: &str,
        content: &str,
        recipient: &str,
        subject: Option<&str>,
    ) -> Result<(), JsPluginError> {
        let channel = self
            .get_channel(channel_name)
            .ok_or_else(|| JsPluginError::Channel(format!("Channel not found: {}", channel_name)))?;

        let message = if let Some(subj) = subject {
            SendMessage::with_subject(content, recipient, subj)
        } else {
            SendMessage::new(content, recipient)
        };

        channel
            .send(&message)
            .await
            .map_err(|e| JsPluginError::Channel(format!("Send failed: {}", e)))
    }

    /// Check if a channel is available
    pub fn has_channel(&self, name: &str) -> bool {
        self.get_channel(name).is_some()
    }

    /// Get the list of available channel names
    pub fn channel_names(&self) -> Vec<String> {
        self.allowed_channels
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get the number of allowed channels
    pub fn len(&self) -> usize {
        self.allowed_channels.len()
    }

    /// Check if any channels are allowed
    pub fn is_empty(&self) -> bool {
        self.allowed_channels.is_empty()
    }
}

impl Default for JsChannelsBridge {
    fn default() -> Self {
        Self::blocked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock Channel for testing
    struct MockChannel {
        name: String,
        sent_messages: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    }

    impl MockChannel {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                sent_messages: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl Channel for MockChannel {
        fn name(&self) -> &str {
            &self.name
        }

        async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
            let mut msgs = self.sent_messages.lock().unwrap();
            msgs.push((message.recipient.clone(), message.content.clone()));
            Ok(())
        }

        async fn listen(
            &self,
            _tx: tokio::sync::mpsc::Sender<crate::channels::traits::ChannelMessage>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[test]
    fn channels_bridge_blocked_by_default() {
        let bridge = JsChannelsBridge::default();
        assert!(bridge.is_empty());
        assert_eq!(bridge.len(), 0);
    }

    #[test]
    fn channels_bridge_with_allowed_channels() {
        let channel1 = Arc::new(MockChannel::new("discord")) as Arc<dyn Channel>;
        let channel2 = Arc::new(MockChannel::new("telegram")) as Arc<dyn Channel>;

        let bridge = JsChannelsBridge::new(vec![
            ("discord".to_string(), channel1),
            ("telegram".to_string(), channel2),
        ]);

        assert_eq!(bridge.len(), 2);
        assert!(bridge.has_channel("discord"));
        assert!(bridge.has_channel("telegram"));
        assert!(!bridge.has_channel("slack"));

        let names = bridge.channel_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"discord".to_string()));
        assert!(names.contains(&"telegram".to_string()));
    }

    #[tokio::test]
    async fn channels_bridge_get_channel() {
        let mock_channel = Arc::new(MockChannel::new("test-channel")) as Arc<dyn Channel>;
        let bridge = JsChannelsBridge::new(vec![("test-channel".to_string(), mock_channel.clone())]);

        let retrieved = bridge.get_channel("test-channel");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test-channel");

        let not_found = bridge.get_channel("nonexistent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn channels_bridge_send_message() {
        let mock_channel = Arc::new(MockChannel::new("test-channel")) as Arc<dyn Channel>;
        let bridge =
            JsChannelsBridge::new(vec![("test-channel".to_string(), mock_channel.clone())]);

        let result = bridge
            .send_message("test-channel", "Hello, world!", "user_123")
            .await;

        assert!(result.is_ok(), "Send should succeed for allowed channel");
    }

    #[tokio::test]
    async fn channels_bridge_send_message_fails_for_unknown_channel() {
        let bridge = JsChannelsBridge::new(vec![]);

        let result = bridge.send_message("unknown", "Hello", "user_123").await;

        assert!(result.is_err());
        match result {
            Err(JsPluginError::Channel(msg)) if msg.contains("not found") => {
                // Expected
            }
            _ => panic!("Expected Channel not found error"),
        }
    }

    #[tokio::test]
    async fn channels_bridge_send_message_with_subject() {
        let mock_channel = Arc::new(MockChannel::new("test-channel")) as Arc<dyn Channel>;
        let bridge =
            JsChannelsBridge::new(vec![("test-channel".to_string(), mock_channel.clone())]);

        let result = bridge
            .send_message_with_subject("test-channel", "Hello, world!", "user_123", Some("Test"))
            .await;

        assert!(result.is_ok(), "Send with subject should succeed");
    }

    #[tokio::test]
    async fn channels_bridge_send_message_without_subject() {
        let mock_channel = Arc::new(MockChannel::new("test-channel")) as Arc<dyn Channel>;
        let bridge =
            JsChannelsBridge::new(vec![("test-channel".to_string(), mock_channel.clone())]);

        let result = bridge
            .send_message_with_subject("test-channel", "Hello", "user_123", None)
            .await;

        assert!(result.is_ok(), "Send without subject should succeed");
    }

    #[tokio::test]
    async fn channels_bridge_channel_names() {
        let channel1 = Arc::new(MockChannel::new("discord")) as Arc<dyn Channel>;
        let channel2 = Arc::new(MockChannel::new("telegram")) as Arc<dyn Channel>;

        let bridge = JsChannelsBridge::new(vec![
            ("discord".to_string(), channel1),
            ("telegram".to_string(), channel2),
        ]);

        let names = bridge.channel_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"discord".to_string()));
        assert!(names.contains(&"telegram".to_string()));
    }

    #[test]
    fn channels_bridge_empty() {
        let bridge = JsChannelsBridge::new(vec![]);
        assert!(bridge.is_empty());
        assert_eq!(bridge.len(), 0);
        assert!(!bridge.has_channel("anything"));
    }
}
