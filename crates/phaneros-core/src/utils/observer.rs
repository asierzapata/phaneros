use std::collections::HashMap;
use std::hash::Hash;

/// A subscriber (listener) has type of a callable function.
pub type Subscriber = fn(file_path: &str);

/// Publisher sends events to subscribers (listeners).
#[derive(Debug, Default)]
pub struct Publisher<E> {
    events: HashMap<E, Vec<Subscriber>>,
}

impl<E: Eq + Hash> Publisher<E> {
    pub fn new() -> Self {
        Publisher {
            events: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, event_type: E, listener: Subscriber) {
        self.events.entry(event_type).or_default().push(listener);
    }

    pub fn unsubscribe(&mut self, event_type: &E, listener: Subscriber) {
        if let Some(listeners) = self.events.get_mut(event_type) {
            listeners.retain(|&x| !std::ptr::fn_addr_eq(x, listener));
            if listeners.is_empty() {
                self.events.remove(event_type);
            }
        }
    }

    // TODO: Make notify accept an arbitrary payload, file path is just a placeholder for now.
    pub fn notify(&self, event_type: &E, file_path: &str) {
        if let Some(listeners) = self.events.get(event_type) {
            for listener in listeners {
                listener(file_path);
            }
        }
    }
}
