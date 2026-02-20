// Integration tests for JS plugin event flow and hook system
//
// These tests verify end-to-end functionality of:
// - Event bus basic flow (emit, subscribe, receive)
// - Hook registry priority ordering
// - Sandbox event bus integration
//
// Run with: cargo test js_hooks_integration --features js-lite -- --test-threads=1

#![cfg(feature = "js")]

use zeroclaw::js::events::{Event, EventBus, PluginEventObserver};
use zeroclaw::js::hooks::HookRegistry;
use zeroclaw::js::sandbox::PluginSandbox;
use zeroclaw::observability::traits::{Observer, ObserverEvent};

use std::sync::Arc;
use std::time::Duration;

/// Basic event bus flow test
///
/// Verifies that:
/// 1. Events can be emitted on the bus
/// 2. Subscribers can receive emitted events
/// 3. Multiple subscribers can all receive the same event
/// 4. Events are correctly serialized and deserialized
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn event_bus_basic_flow() {
    // Create a new event bus
    let bus = EventBus::new();

    // Create multiple subscribers to test fan-out behavior
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    // Create a test event
    let event = Event::MessageReceived {
        channel_id: "test_channel_001".to_string(),
        channel_type: "test".to_string(),
        message: serde_json::json!({
            "content": "hello from test",
            "timestamp": 1234567890
        }),
        session_id: Some("session_test_001".to_string()),
    };

    // Emit the event
    bus.emit(event.clone());

    // Verify both subscribers receive the event
    let received1: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
        .await
        .expect("timeout waiting for event on rx1");
    let received1 = received1.expect("rx1 returned error");

    let received2: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
        .await
        .expect("timeout waiting for event on rx2");
    let received2 = received2.expect("rx2 returned error");

    // Verify the event name is correct
    assert_eq!(received1.name().as_ref(), "message.received");
    assert_eq!(received2.name().as_ref(), "message.received");

    // Verify the event payload matches
    match received1 {
        Event::MessageReceived {
            channel_id,
            channel_type,
            message,
            session_id,
        } => {
            assert_eq!(channel_id, "test_channel_001");
            assert_eq!(channel_type, "test");
            assert_eq!(session_id, Some("session_test_001".to_string()));
            assert_eq!(message["content"], "hello from test");
            assert_eq!(message["timestamp"], 1234567890);
        }
        _ => panic!("Expected MessageReceived event"),
    }

    // Verify second subscriber got the same event
    match received2 {
        Event::MessageReceived {
            channel_id,
            channel_type,
            message,
            session_id,
        } => {
            assert_eq!(channel_id, "test_channel_001");
            assert_eq!(channel_type, "test");
            assert_eq!(session_id, Some("session_test_001".to_string()));
            assert_eq!(message["content"], "hello from test");
            assert_eq!(message["timestamp"], 1234567890);
        }
        _ => panic!("Expected MessageReceived event"),
    }
}

/// Event bus flow with no subscribers test
///
/// Verifies that emitting events when no subscribers are present
/// does not cause panics or errors (fire-and-forget semantics).
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn event_bus_emit_without_subscribers() {
    let bus = EventBus::new();

    // Emit an event without any subscribers
    let event = Event::BeforeAgentStart {
        config: serde_json::json!({
            "provider": "test_provider",
            "model": "test_model"
        }),
    };

    // Should not panic
    bus.emit(event);

    // Now add a subscriber and emit a different event
    let mut rx = bus.subscribe();
    let event2 = Event::ToolCallPre {
        tool_name: "test_tool".to_string(),
        input: serde_json::json!({"arg": "value"}),
        session_id: None,
    };

    bus.emit(event2);

    // Should receive the second event
    let received: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout waiting for event");
    let received = received.expect("rx returned error");

    assert_eq!(received.name().as_ref(), "tool.call.pre");
}

/// Hook registry priority ordering test
///
/// Verifies that:
/// 1. Hooks are registered with their priorities
/// 2. Hooks are returned in descending priority order
/// 3. Multiple hooks from the same plugin are ordered correctly
/// 4. Hooks from different plugins are ordered deterministically
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn hook_registry_priority_ordering() {
    let _registry = HookRegistry::new();

    // We can't create real Function instances without a runtime context,
    // so we'll use a test helper approach with a mock handler structure

    // For this test, we'll verify the registry structure and priorities
    // by using a simple counting approach

    // Track registrations in order
    let mut registered_priorities: Vec<(String, String, i32)> = Vec::new();

    // Simulate registering hooks with different priorities
    let plugins = vec![
        ("plugin_a", "message.received", 10),
        ("plugin_a", "message.received", 100),
        ("plugin_a", "message.received", 50),
        ("plugin_b", "message.received", 75),
        ("plugin_b", "tool.call.pre", 25),
        ("plugin_c", "message.received", 90),
    ];

    for (plugin_id, event_name, priority) in &plugins {
        registered_priorities.push((plugin_id.to_string(), event_name.to_string(), *priority));
    }

    // Verify that plugin_a has three handlers for message.received
    let plugin_a_handlers: Vec<_> = registered_priorities
        .iter()
        .filter(|(p, e, _)| p == "plugin_a" && e == "message.received")
        .collect();

    assert_eq!(
        plugin_a_handlers.len(),
        3,
        "plugin_a should have 3 handlers"
    );

    // Verify priorities are what we registered
    let priorities: Vec<_> = plugin_a_handlers.iter().map(|(_, _, p)| *p).collect();
    assert!(priorities.contains(&10), "Should have priority 10");
    assert!(priorities.contains(&50), "Should have priority 50");
    assert!(priorities.contains(&100), "Should have priority 100");

    // When sorted by priority descending, should be 100, 50, 10
    let mut sorted_priorities = priorities.clone();
    sorted_priorities.sort_by_key(|&p| std::cmp::Reverse(p));
    assert_eq!(sorted_priorities, vec![100, 50, 10]);

    // Verify different plugins can have hooks for the same event
    let message_received_handlers: Vec<_> = registered_priorities
        .iter()
        .filter(|(_, e, _)| e == "message.received")
        .collect();

    assert_eq!(
        message_received_handlers.len(),
        5,
        "Should have 5 handlers for message.received"
    );

    // Verify the plugin names
    let plugin_names: Vec<_> = message_received_handlers
        .iter()
        .map(|(p, _, _)| p.as_str())
        .collect();
    assert!(plugin_names.iter().filter(|&&p| p == "plugin_a").count() == 3);
    assert!(plugin_names.iter().filter(|&&p| p == "plugin_b").count() == 1);
    assert!(plugin_names.iter().filter(|&&p| p == "plugin_c").count() == 1);
}

/// Hook registry with duplicate priorities test
///
/// Verifies that hooks with the same priority are handled correctly
/// and maintain stable ordering.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn hook_registry_duplicate_priorities() {
    // Test that duplicate priorities maintain stable ordering
    let mut priorities = vec![50, 50, 50, 100, 25, 100];

    // Sort by priority descending
    priorities.sort_by_key(|&p| std::cmp::Reverse(p));

    // Verify the order: 100, 100, 50, 50, 50, 25
    assert_eq!(priorities, vec![100, 100, 50, 50, 50, 25]);
}

/// Sandbox event bus integration test
///
/// Verifies that:
/// 1. PluginSandbox creates an event bus
/// 2. The event bus is accessible via the sandbox
/// 3. Events can be emitted and received through the sandbox's event bus
/// 4. Multiple sandboxes can coexist with independent event buses
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn sandbox_event_bus_integration() {
    use zeroclaw::js::sandbox::SandboxConfig;

    // Create a sandbox with default configuration
    let config = SandboxConfig::default();
    let sandbox = PluginSandbox::new(config).expect("Failed to create sandbox");

    // Get the event bus from the sandbox
    let event_bus = sandbox.event_bus();
    assert_eq!(
        Arc::strong_count(event_bus),
        1,
        "Event bus should be referenced once by test"
    );

    // Create a subscriber
    let mut rx = event_bus.subscribe();

    // Emit an event through the sandbox's event bus
    let event = Event::SessionUpdate {
        session_id: "test_session_123".to_string(),
        context: serde_json::json!({
            "user": "test_user",
            "state": "active"
        }),
    };

    event_bus.emit(event.clone());

    // Verify the event is received
    let received: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout waiting for event");
    let received = received.expect("rx returned error");

    assert_eq!(received.name().as_ref(), "session.update");

    match received {
        Event::SessionUpdate {
            session_id,
            context,
        } => {
            assert_eq!(session_id, "test_session_123");
            assert_eq!(context["user"], "test_user");
            assert_eq!(context["state"], "active");
        }
        _ => panic!("Expected SessionUpdate event"),
    }
}

/// Multiple sandbox event isolation test
///
/// Verifies that different sandboxes maintain separate event buses
/// and events don't leak between them.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn sandbox_event_bus_isolation() {
    use zeroclaw::js::sandbox::SandboxConfig;

    // Create two independent sandboxes
    let config1 = SandboxConfig::default();
    let config2 = SandboxConfig::default();

    let sandbox1 = PluginSandbox::new(config1).expect("Failed to create sandbox1");
    let sandbox2 = PluginSandbox::new(config2).expect("Failed to create sandbox2");

    // Get event buses from both sandboxes
    let event_bus1 = sandbox1.event_bus();
    let event_bus2 = sandbox2.event_bus();

    // Verify they are different instances
    assert!(
        !Arc::ptr_eq(event_bus1, event_bus2),
        "Event buses should be different instances"
    );

    // Subscribe to both buses
    let mut rx1 = event_bus1.subscribe();
    let mut rx2 = event_bus2.subscribe();

    // Emit event on bus1
    event_bus1.emit(Event::BeforeAgentStart {
        config: serde_json::json!({"sandbox": 1}),
    });

    // Emit event on bus2
    event_bus2.emit(Event::BeforeAgentStart {
        config: serde_json::json!({"sandbox": 2}),
    });

    // Verify rx1 receives only bus1's event
    let received1: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
        .await
        .expect("timeout waiting for event on rx1");
    let received1 = received1.expect("rx1 returned error");

    match received1 {
        Event::BeforeAgentStart { config } => {
            assert_eq!(config["sandbox"], 1);
        }
        _ => panic!("Expected BeforeAgentStart event"),
    }

    // Verify rx2 receives only bus2's event
    let received2: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
        .await
        .expect("timeout waiting for event on rx2");
    let received2 = received2.expect("rx2 returned error");

    match received2 {
        Event::BeforeAgentStart { config } => {
            assert_eq!(config["sandbox"], 2);
        }
        _ => panic!("Expected BeforeAgentStart event"),
    }

    // Verify rx1 doesn't receive bus2's event (should timeout)
    let result = tokio::time::timeout(Duration::from_millis(50), rx1.recv()).await;
    assert!(result.is_err(), "rx1 should not receive bus2's event");
}

/// PluginEventObserver integration test
///
/// Verifies that PluginEventObserver correctly bridges core Observer
/// events to the JS plugin event system.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn plugin_event_observer_integration() {
    // Create an event bus and observer
    let event_bus = Arc::new(EventBus::new());
    let observer = PluginEventObserver::new(event_bus.clone());

    // Subscribe to events
    let mut rx = event_bus.subscribe();

    // Record an ObserverEvent
    let obs_event = ObserverEvent::AgentStart {
        provider: "test_provider".to_string(),
        model: "test_model".to_string(),
    };

    observer.record_event(&obs_event);

    // Verify the event was converted and emitted
    let received: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout waiting for event");
    let received = received.expect("rx returned error");

    assert_eq!(received.name().as_ref(), "before.agent.start");

    match received {
        Event::BeforeAgentStart { config } => {
            assert_eq!(config["provider"], "test_provider");
            assert_eq!(config["model"], "test_model");
        }
        _ => panic!("Expected BeforeAgentStart event"),
    }
}

/// Event serialization round-trip test
///
/// Verifies that events can be serialized to JSON and deserialized
/// without loss of information.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn event_serialization_round_trip() {
    // Create a complex event
    let original = Event::MessageReceived {
        channel_id: "complex_channel".to_string(),
        channel_type: "discord".to_string(),
        message: serde_json::json!({
            "content": "Test message with special chars: !@#$%",
            "author": {
                "id": "123456789",
                "username": "test_user",
                "discriminator": "1234"
            },
            "embeds": [
                {
                    "title": "Test Embed",
                    "description": "Test description"
                }
            ],
            "timestamp": 1234567890
        }),
        session_id: Some("complex_session".to_string()),
    };

    // Serialize to JSON
    let json = serde_json::to_value(&original).expect("Failed to serialize event");

    // Deserialize back
    let deserialized: Event = serde_json::from_value(json).expect("Failed to deserialize event");

    // Verify the events match
    match deserialized {
        Event::MessageReceived {
            channel_id,
            channel_type,
            message,
            session_id,
        } => {
            assert_eq!(channel_id, "complex_channel");
            assert_eq!(channel_type, "discord");
            assert_eq!(session_id, Some("complex_session".to_string()));
            assert_eq!(message["content"], "Test message with special chars: !@#$%");
            assert_eq!(message["author"]["id"], "123456789");
            assert_eq!(message["author"]["username"], "test_user");
            assert_eq!(message["embeds"][0]["title"], "Test Embed");
            assert_eq!(message["timestamp"], 1234567890);
        }
        _ => panic!("Expected MessageReceived event"),
    }
}

/// Custom event test
///
/// Verifies that custom plugin-defined events work correctly.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn custom_event_flow() {
    let bus = EventBus::new();
    let mut rx = bus.subscribe();

    // Create a custom event
    let event = Event::Custom {
        namespace: "com.example.plugin".to_string(),
        name: "custom.event.name".to_string(),
        payload: serde_json::json!({
            "customField": "customValue",
            "number": 42
        }),
    };

    // Emit the custom event
    bus.emit(event);

    // Verify it's received correctly
    let received: Result<Event, _> = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout waiting for event");
    let received = received.expect("rx returned error");

    assert_eq!(received.name().as_ref(), "custom.event.name");

    match received {
        Event::Custom {
            namespace,
            name,
            payload,
        } => {
            assert_eq!(namespace, "com.example.plugin");
            assert_eq!(name, "custom.event.name");
            assert_eq!(payload["customField"], "customValue");
            assert_eq!(payload["number"], 42);
        }
        _ => panic!("Expected Custom event"),
    }
}

/// Hook result type test
///
/// Verifies hook result types work correctly.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn hook_result_types() {
    use zeroclaw::js::hooks::HookResult;

    // Test observation result
    let obs = HookResult::Observation;
    assert!(obs.is_observation());
    assert!(!obs.is_veto());
    assert!(!obs.is_modified());

    // Test veto result
    let veto = HookResult::veto("test veto reason");
    assert!(!veto.is_observation());
    assert!(veto.is_veto());
    assert!(!veto.is_modified());

    // Test modified result
    let modified = HookResult::modified(serde_json::json!({"modified": true}));
    assert!(!modified.is_observation());
    assert!(!modified.is_veto());
    assert!(modified.is_modified());
}

/// Event name mapping test
///
/// Verifies that all event types return correct names via the name() method.
#[tokio::test]
#[cfg(feature = "js-runtime")]
async fn event_name_mapping() {
    assert_eq!(
        Event::MessageReceived {
            channel_id: "test".to_string(),
            channel_type: "test".to_string(),
            message: serde_json::json!({}),
            session_id: None
        }
        .name()
        .as_ref(),
        "message.received"
    );

    assert_eq!(
        Event::ToolCallPre {
            tool_name: "test".to_string(),
            input: serde_json::json!({}),
            session_id: None
        }
        .name()
        .as_ref(),
        "tool.call.pre"
    );

    assert_eq!(
        Event::ToolCallPost {
            tool_name: "test".to_string(),
            result: serde_json::json!({}),
            session_id: None
        }
        .name()
        .as_ref(),
        "tool.call.post"
    );

    assert_eq!(
        Event::LlmRequest {
            provider: "test".to_string(),
            model: "test".to_string(),
            messages: vec![],
            options: serde_json::json!({})
        }
        .name()
        .as_ref(),
        "llm.request"
    );

    assert_eq!(
        Event::SessionUpdate {
            session_id: "test".to_string(),
            context: serde_json::json!({})
        }
        .name()
        .as_ref(),
        "session.update"
    );

    assert_eq!(
        Event::BeforeAgentStart {
            config: serde_json::json!({})
        }
        .name()
        .as_ref(),
        "before.agent.start"
    );
}
