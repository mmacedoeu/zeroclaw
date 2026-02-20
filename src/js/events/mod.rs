// Event types and bus for JS plugin hook system
//
// This module provides the core event system for ZeroClaw's JS plugin hooks.
// Events represent lifecycle and runtime moments that plugins can observe.

mod bus;
mod event;
mod observer;

pub use bus::{EventBus, EventReceiver, EventSender};
pub use event::Event;
pub use observer::PluginEventObserver;
