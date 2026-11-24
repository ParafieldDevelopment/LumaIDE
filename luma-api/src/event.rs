//! A Bukkit-style event bus shared by the IDE and its extensions.
//!
//! Handlers are ordered by [`EventPriority`] (`Lowest` first, `Monitor` last),
//! can cancel an event through [`EventContext`], and may opt out of cancelled
//! events with `ignore_cancelled` — the same semantics as Bukkit's
//! `@EventHandler(priority = ..., ignoreCancelled = ...)`.
//!
//! Events are also addressable by name and JSON payload. That is what lets
//! native and WASM extensions participate in the same bus: a host event is
//! serialized and forwarded to a guest listener, while events fired by a guest
//! are dispatched through the host's bus. See [`EventBus::register_named`].
//!
//! ```
//! use luma_api::event::{Event, EventBus, EventContext, EventPriority};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct MyEvent { message: String }
//!
//! impl Event for MyEvent {
//!     fn name() -> &'static str { "my-extension:my-event" }
//! }
//!
//! let bus = EventBus::new();
//! bus.register::<MyEvent, _>("my-extension", EventPriority::Normal, false,
//!     |event: &MyEvent, context: &mut EventContext| {
//!         if event.message.is_empty() {
//!             context.set_cancelled(true);
//!         }
//!     });
//! let outcome = bus.fire(MyEvent { message: String::new() });
//! assert!(outcome.cancelled);
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// An event that can be dispatched on an [`EventBus`].
///
/// `name` is the stable, namespaced identifier used by extensions and should
/// never change once an event is published. Serialization is required because
/// named handlers (including extension listeners) receive a JSON payload.
pub trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn name() -> &'static str;
}

/// Handler order, equivalent to Bukkit's `EventPriority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum EventPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    /// Observers run last and must not change cancellation or propagation.
    Monitor = 5,
}

impl EventPriority {
    /// Convert an ABI priority code, clamping unknown values to `Monitor`.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Lowest,
            1 => Self::Low,
            2 => Self::Normal,
            3 => Self::High,
            4 => Self::Highest,
            _ => Self::Monitor,
        }
    }
}

impl From<EventPriority> for u8 {
    fn from(value: EventPriority) -> Self {
        value as u8
    }
}

/// Token returned by handler registration, used to unregister that handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(u64);

impl HandlerId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for HandlerId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// Result of dispatching one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventOutcome {
    /// Whether a handler cancelled the event (including a bridged guest).
    pub cancelled: bool,
    /// Number of local handlers invoked. Bridged handlers are not counted.
    pub handlers_called: usize,
}

impl EventOutcome {
    pub fn new(cancelled: bool, handlers_called: usize) -> Self {
        Self {
            cancelled,
            handlers_called,
        }
    }
}

/// Mutable state shared by all handlers of a single dispatch.
#[derive(Debug, Clone, Copy)]
pub struct EventContext {
    cancelled: bool,
    propagation_stopped: bool,
}

impl EventContext {
    fn new(cancelled: bool) -> Self {
        Self {
            cancelled,
            propagation_stopped: false,
        }
    }

    /// Mark the event as cancelled. Later handlers with `ignore_cancelled`
    /// will be skipped.
    pub fn set_cancelled(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Skip all remaining handlers for this dispatch.
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}

/// Connects a guest-side bus to the host's bus. Implemented by the native and
/// WASM extension bridges; host code uses an [`EventBus`] without a bridge.
pub trait EventBridge: Send + Sync {
    /// Ask the host to register a listener for `name`. `None` means the host
    /// cannot bridge this event and the handler stays local as a fallback.
    fn register(&self, name: &str, priority: EventPriority, ignore_cancelled: bool) -> Option<u64>;

    fn unregister(&self, remote_id: u64);

    /// Dispatch an event on the host bus, returning the final cancelled flag.
    fn fire(&self, name: &str, payload: &Value, cancelled: bool) -> bool;
}

type TypedHandler = Arc<dyn Fn(&dyn Any, &mut EventContext) + Send + Sync>;
type NamedHandler = Arc<dyn Fn(&Value, &mut EventContext) + Send + Sync>;
type EventDeserializer = fn(&Value) -> Option<Box<dyn Any + Send + Sync>>;

#[derive(Clone)]
enum HandlerKind {
    Typed(TypedHandler),
    Named(NamedHandler),
}

#[derive(Clone)]
struct Handler {
    id: HandlerId,
    owner: String,
    priority: EventPriority,
    ignore_cancelled: bool,
    kind: HandlerKind,
}

#[derive(Default)]
struct HandlerList {
    handlers: Vec<Handler>,
}

impl HandlerList {
    fn insert(&mut self, handler: Handler) {
        self.handlers.push(handler);
        self.handlers
            .sort_by_key(|handler| (handler.priority, handler.id.as_u64()));
    }
}

/// A synchronous, panic-isolated event bus.
///
/// Handlers may register or unregister other handlers while an event is being
/// dispatched: each dispatch works on a snapshot of the handler list.
pub struct EventBus {
    handlers: RwLock<HashMap<String, HandlerList>>,
    deserializers: RwLock<HashMap<String, (TypeId, EventDeserializer)>>,
    /// Remote id and owner for handlers that live on the other side of a bridge.
    bridge_ids: RwLock<HashMap<HandlerId, (u64, String)>>,
    next_id: AtomicU64,
    bridge: Option<Arc<dyn EventBridge>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            deserializers: RwLock::new(HashMap::new()),
            bridge_ids: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            bridge: None,
        }
    }

    /// Build a bus whose handlers and fired events are relayed through `bridge`.
    /// Used by extension guests; the host bus is always local.
    pub fn with_bridge(bridge: Arc<dyn EventBridge>) -> Self {
        Self {
            bridge: Some(bridge),
            ..Self::new()
        }
    }

    /// Register a strongly typed handler for `E`.
    ///
    /// `owner` identifies the plugin/extension/module that registered it and
    /// allows bulk removal with [`Self::unregister_owner`].
    pub fn register<E, F>(
        &self,
        owner: impl Into<String>,
        priority: EventPriority,
        ignore_cancelled: bool,
        handler: F,
    ) -> HandlerId
    where
        E: Event,
        F: Fn(&E, &mut EventContext) + Send + Sync + 'static,
    {
        self.register_deserializer::<E>(E::name());
        self.add_handler(
            E::name().parse().unwrap(),
            owner,
            priority,
            ignore_cancelled,
            HandlerKind::Typed(Arc::new(move |event, context| {
                if let Some(event) = event.downcast_ref::<E>() {
                    handler(event, context);
                }
            })),
        )
    }

    /// Register a handler that receives the event as a JSON value, regardless
    /// of the concrete Rust type. This is the extension-facing API.
    pub fn register_named<F>(
        &self,
        name: impl Into<String>,
        owner: impl Into<String>,
        priority: EventPriority,
        ignore_cancelled: bool,
        handler: F,
    ) -> HandlerId
    where
        F: Fn(&Value, &mut EventContext) + Send + Sync + 'static,
    {
        self.add_handler(
            name.into(),
            owner,
            priority,
            ignore_cancelled,
            HandlerKind::Named(Arc::new(handler)),
        )
    }

    fn register_deserializer<E: Event>(&self, name: &'static str) {
        let Ok(mut deserializers) = self.deserializers.write() else {
            log::warn!("Event deserializer lock was poisoned");
            return;
        };
        deserializers.entry(name.parse().unwrap()).or_insert((
            TypeId::of::<E>(),
            |value| serde_json::from_value::<E>(value.clone()).ok().map(|event| Box::new(event) as Box<dyn Any + Send + Sync>),
        ));
    }

    fn add_handler(
        &self,
        name: String,
        owner: impl Into<String>,
        priority: EventPriority,
        ignore_cancelled: bool,
        kind: HandlerKind,
    ) -> HandlerId {
        let id = HandlerId(self.next_id.fetch_add(1, Ordering::Relaxed));

        // A bridged guest registers its listener once on the host bus. The
        // guest's local bus only invokes it when the host dispatches back into
        // the guest, so events fired by the guest are not delivered twice.
        if let Some(bridge) = &self.bridge
            && let Some(remote_id) = bridge.register(&name, priority, ignore_cancelled)
        {
            if let Ok(mut ids) = self.bridge_ids.write() {
                ids.insert(id, (remote_id, owner.into()));
            }
            return id;
        }

        if let Ok(mut handlers) = self.handlers.write() {
            handlers.entry(name).or_default().insert(Handler {
                id,
                owner: owner.into(),
                priority,
                ignore_cancelled,
                kind,
            });
        } else {
            log::warn!("Event handler lock was poisoned while registering");
        }
        id
    }

    /// Remove one handler, locally and on the host if it was bridged.
    pub fn unregister(&self, id: HandlerId) -> bool {
        let remote_id = self
            .bridge_ids
            .write()
            .ok()
            .and_then(|mut ids| ids.remove(&id));
        let mut removed = false;
        if let (Some(bridge), Some((remote_id, _))) = (&self.bridge, remote_id) {
            bridge.unregister(remote_id);
            removed = true;
        }

        let Ok(mut handlers) = self.handlers.write() else {
            return removed;
        };
        for list in handlers.values_mut() {
            let before = list.handlers.len();
            list.handlers.retain(|handler| handler.id != id);
            if list.handlers.len() != before {
                removed = true;
            }
        }
        removed
    }

    /// Remove every handler owned by `owner`. Used when an extension unloads.
    pub fn unregister_owner(&self, owner: &str) -> usize {
        let mut ids: Vec<HandlerId> = self
            .handlers
            .read()
            .ok()
            .map(|handlers| {
                handlers
                    .values()
                    .flat_map(|list| list.handlers.iter())
                    .filter(|handler| handler.owner == owner)
                    .map(|handler| handler.id)
                    .collect()
            })
            .unwrap_or_default();
        if let Ok(bridge_ids) = self.bridge_ids.read() {
            ids.extend(
                bridge_ids
                    .iter()
                    .filter(|(_, (_, handler_owner))| handler_owner == owner)
                    .map(|(id, _)| *id),
            );
        }
        ids.sort_unstable_by_key(|id| id.as_u64());
        ids.dedup();
        let count = ids.len();
        for id in ids {
            self.unregister(id);
        }
        count
    }

    /// Dispatch a typed event to local handlers (and through the bridge).
    pub fn fire<E: Event>(&self, event: E) -> EventOutcome {
        let name = E::name();
        let payload = serde_json::to_value(&event).unwrap_or_else(|error| {
            log::warn!("Failed to serialize event '{name}': {error}");
            Value::Null
        });
        self.dispatch(name, &payload, Some(&event), false, true)
    }

    /// Dispatch an event by name and JSON payload.
    pub fn fire_named(&self, name: &str, payload: Value) -> EventOutcome {
        let typed = self.typed_from_json(name, &payload);
        let typed = typed.as_deref().map(|value| value as &dyn Any);
        self.dispatch(name, &payload, typed, false, true)
    }

    /// Dispatch into local handlers with a pre-set cancellation flag, without
    /// forwarding through the bridge. Event bridges use this to deliver host
    /// events to guest listeners without creating a dispatch loop.
    pub fn fire_named_seeded(
        &self,
        name: &str,
        payload: Value,
        initial_cancelled: bool,
    ) -> EventOutcome {
        let typed = self.typed_from_json(name, &payload);
        let typed = typed.as_deref().map(|value| value as &dyn Any);
        self.dispatch(name, &payload, typed, initial_cancelled, false)
    }

    /// Number of local handlers registered for an event name.
    pub fn handler_count(&self, name: &str) -> usize {
        self.handlers
            .read()
            .map(|handlers| {
                handlers
                    .get(name)
                    .map(|list| list.handlers.len())
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    fn typed_from_json(&self, name: &str, payload: &Value) -> Option<Box<dyn Any + Send + Sync>> {
        let deserializer = self
            .deserializers
            .read()
            .ok()?
            .get(name)
            .map(|(_, deserializer)| *deserializer)?;
        deserializer(payload)
    }

    fn dispatch(
        &self,
        name: &str,
        payload: &Value,
        typed: Option<&dyn Any>,
        initial_cancelled: bool,
        allow_bridge: bool,
    ) -> EventOutcome {
        let mut context = EventContext::new(initial_cancelled);
        let handlers = self
            .handlers
            .read()
            .ok()
            .and_then(|handlers| handlers.get(name).map(|list| list.handlers.clone()))
            .unwrap_or_default();

        let mut called = 0;
        for handler in handlers {
            if context.is_propagation_stopped() {
                break;
            }
            if handler.ignore_cancelled && context.is_cancelled() {
                continue;
            }
            let cancelled_before = context.cancelled;
            let stopped_before = context.propagation_stopped;
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| match &handler.kind {
                HandlerKind::Typed(callback) => callback(typed.unwrap_or(&()), &mut context),
                HandlerKind::Named(callback) => callback(payload, &mut context),
            }));
            if outcome.is_err() {
                log::warn!(
                    "Event handler for '{name}' owned by '{}' panicked",
                    handler.owner
                );
            }
            // Bukkit's MONITOR handlers observe the final state but may not
            // change it.
            if handler.priority == EventPriority::Monitor {
                context.cancelled = cancelled_before;
                context.propagation_stopped = stopped_before;
            }
            called += 1;
        }

        let bridge_allowed = allow_bridge && !context.is_propagation_stopped();
        if let (true, Some(bridge)) = (bridge_allowed, self.bridge.as_ref()) {
            let cancelled = bridge.fire(name, payload, context.is_cancelled());
            context.set_cancelled(cancelled);
        }

        EventOutcome::new(context.is_cancelled(), called)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::sync::Mutex;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        value: String,
    }

    impl Event for TestEvent {
        fn name() -> &'static str {
            "test:event"
        }
    }

    fn recording_bus(order: Arc<Mutex<Vec<&'static str>>>) -> EventBus {
        let bus = EventBus::new();
        for (owner, priority) in [
            ("normal", EventPriority::Normal),
            ("lowest", EventPriority::Lowest),
            ("monitor", EventPriority::Monitor),
            ("highest", EventPriority::Highest),
        ] {
            let order = Arc::clone(&order);
            bus.register::<TestEvent, _>(owner, priority, false, move |_, _| {
                order.lock().unwrap().push(owner);
            });
        }
        bus
    }

    #[test]
    fn handlers_run_in_bukkit_priority_order() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let bus = recording_bus(Arc::clone(&order));
        bus.fire(TestEvent {
            value: "hello".into(),
        });
        assert_eq!(
            *order.lock().unwrap(),
            vec!["lowest", "normal", "highest", "monitor"]
        );
    }

    #[test]
    fn cancellation_and_ignore_cancelled_match_bukkit() {
        let bus = EventBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));

        let seen_by_handler = Arc::clone(&seen);
        bus.register::<TestEvent, _>("canceller", EventPriority::High, false, move |_, ctx| {
            ctx.set_cancelled(true);
            seen_by_handler.lock().unwrap().push("canceller");
        });
        let seen_by_handler = Arc::clone(&seen);
        bus.register::<TestEvent, _>(
            "observer",
            EventPriority::Highest,
            true,
            move |_, _| {
                seen_by_handler.lock().unwrap().push("observer");
            },
        );
        let seen_by_handler = Arc::clone(&seen);
        bus.register::<TestEvent, _>("monitor", EventPriority::Monitor, false, move |_, ctx| {
            assert!(ctx.is_cancelled());
            ctx.set_cancelled(false); // Monitors cannot change the outcome.
            seen_by_handler.lock().unwrap().push("monitor");
        });

        let outcome = bus.fire(TestEvent {
            value: "hello".into(),
        });
        assert!(outcome.cancelled);
        assert_eq!(*seen.lock().unwrap(), vec!["canceller", "monitor"]);
    }

    #[test]
    fn named_payloads_reach_typed_handlers() {
        let bus = EventBus::new();
        let message = Arc::new(Mutex::new(String::new()));
        let handler_message = Arc::clone(&message);
        bus.register::<TestEvent, _>("listener", EventPriority::Normal, false, move |event, _| {
            *handler_message.lock().unwrap() = event.value.clone();
        });

        let payload = serde_json::json!({ "value": "from-json" });
        let outcome = bus.fire_named("test:event", payload);
        assert_eq!(outcome.handlers_called, 1);
        assert_eq!(*message.lock().unwrap(), "from-json");
    }

    #[test]
    fn unregister_and_unregister_owner() {
        let bus = EventBus::new();
        let id = bus.register::<TestEvent, _>(
            "owner-a",
            EventPriority::Normal,
            false,
            |_, _| {},
        );
        bus.register::<TestEvent, _>("owner-b", EventPriority::Normal, false, |_, _| {});
        assert_eq!(bus.handler_count("test:event"), 2);
        assert!(bus.unregister(id));
        assert_eq!(bus.handler_count("test:event"), 1);
        assert_eq!(bus.unregister_owner("owner-b"), 1);
        assert_eq!(bus.handler_count("test:event"), 0);
    }

    #[test]
    fn a_panicking_handler_does_not_stop_dispatch() {
        let bus = EventBus::new();
        bus.register::<TestEvent, _>("bad", EventPriority::Lowest, false, |_, _| {
            panic!("handler failed");
        });
        let called = Arc::new(Mutex::new(false));
        let handler_called = Arc::clone(&called);
        bus.register::<TestEvent, _>("good", EventPriority::Normal, false, move |_, _| {
            *handler_called.lock().unwrap() = true;
        });
        let outcome = bus.fire(TestEvent {
            value: "hello".into(),
        });
        assert_eq!(outcome.handlers_called, 2);
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn stop_propagation_skips_remaining_handlers() {
        let bus = EventBus::new();
        bus.register::<TestEvent, _>("stopper", EventPriority::Low, false, |_, ctx| {
            ctx.stop_propagation();
        });
        let called = Arc::new(Mutex::new(false));
        let handler_called = Arc::clone(&called);
        bus.register::<TestEvent, _>("later", EventPriority::Normal, false, move |_, _| {
            *handler_called.lock().unwrap() = true;
        });
        bus.fire(TestEvent {
            value: "hello".into(),
        });
        assert!(!*called.lock().unwrap());
    }

    #[derive(Default)]
    struct MockBridge {
        registrations: Mutex<Vec<String>>,
        fires: Mutex<Vec<(String, Value, bool)>>,
        unregistrations: Mutex<Vec<u64>>,
    }

    impl EventBridge for MockBridge {
        fn register(
            &self,
            name: &str,
            _priority: EventPriority,
            _ignore_cancelled: bool,
        ) -> Option<u64> {
            self.registrations.lock().unwrap().push(name.to_string());
            Some(1)
        }

        fn unregister(&self, remote_id: u64) {
            self.unregistrations.lock().unwrap().push(remote_id);
        }

        fn fire(&self, name: &str, payload: &Value, cancelled: bool) -> bool {
            self.fires
                .lock()
                .unwrap()
                .push((name.to_string(), payload.clone(), cancelled));
            true
        }
    }

    #[test]
    fn bridged_bus_registers_remotely_and_relays_cancellation() {
        let bridge = Arc::new(MockBridge::default());
        let bus = EventBus::with_bridge(Arc::clone(&bridge) as Arc<dyn EventBridge>);

        let id = bus.register::<TestEvent, _>("guest", EventPriority::High, false, |_, _| {});
        // The guest listener lives on the host bus so events fired by the guest
        // are not delivered twice.
        assert_eq!(bus.handler_count("test:event"), 0);
        assert_eq!(*bridge.registrations.lock().unwrap(), vec!["test:event"]);

        let outcome = bus.fire(TestEvent {
            value: "hello".into(),
        });
        assert!(outcome.cancelled);
        assert_eq!(bridge.fires.lock().unwrap().len(), 1);

        // Host events delivered into the guest must not bounce back.
        bus.fire_named_seeded(
            "test:event",
            serde_json::json!({ "value": "hello" }),
            false,
        );
        assert_eq!(bridge.fires.lock().unwrap().len(), 1);

        assert!(bus.unregister(id));
        assert_eq!(*bridge.unregistrations.lock().unwrap(), vec![1]);
    }

    #[test]
    fn seeded_dispatch_starts_cancelled() {
        let bus = EventBus::new();
        let saw_cancelled = Arc::new(Mutex::new(false));
        let handler_flag = Arc::clone(&saw_cancelled);
        bus.register::<TestEvent, _>("listener", EventPriority::Normal, false, move |_, ctx| {
            *handler_flag.lock().unwrap() = ctx.is_cancelled();
        });
        let outcome = bus.fire_named_seeded(
            "test:event",
            serde_json::json!({ "value": "hello" }),
            true,
        );
        assert!(outcome.cancelled);
        assert!(*saw_cancelled.lock().unwrap());
    }
}
