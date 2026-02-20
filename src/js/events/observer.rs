// PluginEventObserver - Bridges core Observer events to JS plugin event system
//
// This module provides the bridge between ZeroClaw's core observability system
// and the JS plugin event system. It implements the Observer trait and converts
// core runtime events into JS plugin events that can be consumed by plugins.

use crate::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use std::sync::Arc;

/// Bridges core Observer events to JS plugin event system
///
/// PluginEventObserver implements the Observer trait and converts internal
/// ZeroClaw runtime events into the Event enum used by the JS plugin system.
/// This allows plugins to hook into core runtime lifecycle moments.
///
/// # Event Mapping
///
/// Only a subset of ObserverEvent variants are mapped to JS plugin events:
/// - `AgentStart` -> `BeforeAgentStart`
/// - `ToolCallStart` -> `ToolCallPre`
///
/// Additional mappings can be added as needed. Events with no mapping
/// are silently ignored (fire-and-forget semantics).
pub struct PluginEventObserver {
    event_bus: Arc<super::EventBus>,
}

impl PluginEventObserver {
    /// Create a new PluginEventObserver
    ///
    /// # Arguments
    /// * `event_bus` - The event bus to emit JS plugin events to
    pub fn new(event_bus: Arc<super::EventBus>) -> Self {
        Self { event_bus }
    }

    /// Convert ObserverEvent to Event when possible
    ///
    /// This method maps core runtime events to their JS plugin event equivalents.
    /// Returns None for events that don't have a mapping (intentionally ignored).
    ///
    /// # Mappings
    ///
    /// | ObserverEvent | Event |
    /// |---------------|-------|
    /// | `AgentStart` | `BeforeAgentStart` |
    /// | `ToolCallStart` | `ToolCallPre` |
    fn convert_event(&self, observer_event: &ObserverEvent) -> Option<crate::js::events::Event> {
        match observer_event {
            ObserverEvent::ToolCallStart { tool } => Some(crate::js::events::Event::ToolCallPre {
                tool_name: tool.clone(),
                input: serde_json::Value::Null,
                session_id: None,
            }),

            ObserverEvent::AgentStart { provider, model } => {
                Some(crate::js::events::Event::BeforeAgentStart {
                    config: serde_json::json!({
                        "provider": provider,
                        "model": model
                    }),
                })
            }

            // Intentionally ignore unmapped events (fire-and-forget semantics)
            _ => None,
        }
    }
}

impl Observer for PluginEventObserver {
    fn record_event(&self, event: &ObserverEvent) {
        if let Some(js_event) = self.convert_event(event) {
            self.event_bus.emit(js_event);
        }
    }

    fn record_metric(&self, _metric: &ObserverMetric) {
        // Metrics not relevant for event bus - intentionally no-op
    }

    fn flush(&self) {
        // No buffering - events are emitted synchronously
    }

    fn name(&self) -> &str {
        "js-plugin-events"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_name() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus);
        assert_eq!(observer.name(), "js-plugin-events");
    }

    #[test]
    fn convert_agent_start_event() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus.clone());

        let obs_event = ObserverEvent::AgentStart {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
        };

        observer.record_event(&obs_event);

        let mut rx = bus.subscribe();
        let received = rx.blocking_recv().unwrap();
        assert_eq!(received.name().as_ref(), "before.agent.start");
    }

    #[test]
    fn convert_tool_call_start_event() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus.clone());

        let obs_event = ObserverEvent::ToolCallStart {
            tool: "shell".to_string(),
        };

        observer.record_event(&obs_event);

        let mut rx = bus.subscribe();
        let received = rx.blocking_recv().unwrap();
        assert_eq!(received.name().as_ref(), "tool.call.pre");
    }

    #[test]
    fn observer_ignores_metrics() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus.clone());

        // Recording a metric should not emit any events
        observer.record_metric(&ObserverMetric::ActiveSessions(5));

        // Emit a marker event to verify no other events were sent first
        bus.emit(crate::js::events::Event::BeforeAgentStart {
            config: serde_json::json!({}),
        });

        // Should only receive the marker event we just emitted
        let mut rx = bus.subscribe();
        let received = rx.blocking_recv().unwrap();
        assert_eq!(received.name().as_ref(), "before.agent.start");

        // Try to receive again - should get RecvError since no more events
        let result = rx.blocking_recv();
        assert!(result.is_err());
    }

    #[test]
    fn observer_flush_no_op() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus);

        // flush should not panic (no-op implementation)
        observer.flush();
    }

    #[test]
    fn observer_as_any() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus);

        // as_any should return self
        assert!(observer.as_any().is::<PluginEventObserver>());
    }

    #[test]
    fn unmapped_events_ignored() {
        let bus = Arc::new(super::super::EventBus::new());
        let observer = PluginEventObserver::new(bus.clone());

        // These events have no mapping and should be ignored
        observer.record_event(&ObserverEvent::HeartbeatTick);
        observer.record_event(&ObserverEvent::TurnComplete);

        // Emit a marker event to verify no other events were sent first
        bus.emit(crate::js::events::Event::BeforeAgentStart {
            config: serde_json::json!({}),
        });

        // Should only receive the marker event we just emitted
        let mut rx = bus.subscribe();
        let received = rx.blocking_recv().unwrap();
        assert_eq!(received.name().as_ref(), "before.agent.start");

        // Try to receive again - should get RecvError since no more events
        let result = rx.blocking_recv();
        assert!(result.is_err());
    }
}
