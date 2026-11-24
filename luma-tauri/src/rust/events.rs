//! Bridge between the shared event bus and the Vue frontend.
//!
//! The frontend can subscribe to any named event (including built-in IDE
//! events and custom events fired by extensions) and can fire events back into
//! the bus. Subscriptions are ordinary named handlers on the process-wide bus,
//! so extensions see frontend-fired events like any other event.
//!
//! Host events are mirrored to the frontend as the Tauri event `luma-event`.
//! Cancellation of an already-running host event is observation-only: the
//! frontend receives the current flag, but the synchronous bus does not wait
//! for a reply. Events fired *from* the frontend do return the final outcome.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use luma_api::event::{EventOutcome, EventPriority, HandlerId};
use luma_api::events::{
    FILE_ACTIVATED_EVENT, FILE_CLOSED_EVENT, FILE_OPENED_EVENT, FileActivatedEvent,
    FileClosedEvent, FileOpenedEvent,
};
use luma_api::event_bus;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, WebviewWindow};
use tauri::ipc::InvokeError;

/// Tauri event name used to deliver bus events to the frontend.
pub const FRONTEND_EVENT: &str = "luma-event";

/// Payload delivered with [`FRONTEND_EVENT`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LumaEventEnvelope {
    pub subscription_id: u64,
    pub name: String,
    pub payload: Value,
    pub cancelled: bool,
}

/// Result returned to the frontend when it fires an event.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FireResult {
    pub cancelled: bool,
    pub handlers_called: usize,
}

impl From<EventOutcome> for FireResult {
    fn from(outcome: EventOutcome) -> Self {
        Self {
            cancelled: outcome.cancelled,
            handlers_called: outcome.handlers_called,
        }
    }
}

/// Handler ids grouped by the window that registered them.
static SUBSCRIPTIONS: std::sync::OnceLock<Mutex<HashMap<String, Vec<HandlerId>>>> =
    std::sync::OnceLock::new();

fn subscriptions() -> &'static Mutex<HashMap<String, Vec<HandlerId>>> {
    SUBSCRIPTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn owner_for(label: &str) -> String {
    format!("frontend:{label}")
}

/// Register a named handler that mirrors bus events into one frontend window.
#[tauri::command]
pub fn event_register(
    app_handle: AppHandle,
    window: WebviewWindow,
    name: String,
    priority: Option<u8>,
    ignore_cancelled: Option<bool>,
) -> Result<u64, InvokeError> {
    let label = window.label().to_string();
    let subscription_id = std::sync::Arc::new(AtomicU64::new(0));
    let id_for_handler = std::sync::Arc::clone(&subscription_id);
    let app_for_handler = app_handle.clone();
    let label_for_handler = label.clone();
    let name_for_handler = name.clone();

    let handler_id = event_bus().register_named(
        name,
        owner_for(&label),
        EventPriority::from_u8(priority.unwrap_or(u8::from(EventPriority::Normal))),
        ignore_cancelled.unwrap_or(false),
        move |payload, context| {
            let envelope = LumaEventEnvelope {
                subscription_id: id_for_handler.load(Ordering::Relaxed),
                name: name_for_handler.clone(),
                payload: payload.clone(),
                cancelled: context.is_cancelled(),
            };
            if let Err(error) = app_for_handler.emit_to(&label_for_handler, FRONTEND_EVENT, envelope)
            {
                log::warn!("Failed to emit bus event to {label_for_handler}: {error}");
            }
        },
    );

    subscription_id.store(handler_id.as_u64(), Ordering::Relaxed);
    if let Ok(mut subscriptions) = subscriptions().lock() {
        subscriptions
            .entry(label)
            .or_default()
            .push(handler_id);
    }
    Ok(handler_id.as_u64())
}

/// Remove one frontend subscription.
#[tauri::command]
pub fn event_unregister(id: u64) -> bool {
    if let Ok(mut subscriptions) = subscriptions().lock() {
        for handlers in subscriptions.values_mut() {
            handlers.retain(|handler| handler.as_u64() != id);
        }
    }
    event_bus().unregister(HandlerId::from(id))
}

/// Fire a named event from the frontend and return its final outcome.
#[tauri::command]
pub fn event_fire(name: String, payload: Option<Value>, cancelled: Option<bool>) -> FireResult {
    event_bus()
        .fire_named_seeded(
            &name,
            payload.unwrap_or(Value::Null),
            cancelled.unwrap_or(false),
        )
        .into()
}

/// Drop every subscription belonging to a window that was destroyed.
pub fn cleanup_window(label: &str) {
    let handlers = subscriptions()
        .lock()
        .ok()
        .and_then(|mut subscriptions| subscriptions.remove(label))
        .unwrap_or_default();
    let bus = event_bus();
    for handler in handlers {
        bus.unregister(handler);
    }
}

// The editor lives in the frontend, so these actions notify the bus from IPC.
// The host-side file/terminal/workspace commands fire their events directly.

#[tauri::command]
pub fn notify_file_opened(window: WebviewWindow, path: String) {
    event_bus().fire(FileOpenedEvent {
        path,
        window: window.label().to_string(),
    });
}

#[tauri::command]
pub fn notify_file_closed(window: WebviewWindow, path: String) {
    event_bus().fire(FileClosedEvent {
        path,
        window: window.label().to_string(),
    });
}

#[tauri::command]
pub fn notify_file_activated(window: WebviewWindow, path: Option<String>) {
    event_bus().fire(FileActivatedEvent {
        path,
        window: window.label().to_string(),
    });
}

// Keep the built-in event names referenced here so a rename in luma-api is a
// compile error in this bridge too.
const _: &str = FILE_OPENED_EVENT;
const _: &str = FILE_CLOSED_EVENT;
const _: &str = FILE_ACTIVATED_EVENT;
