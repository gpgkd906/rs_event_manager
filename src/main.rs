use std::sync::{Arc, Mutex};
use tokio::task;
use std::collections::HashMap;
use icecream::ic;
use lazy_static::lazy_static;

#[derive(Debug, Clone, PartialEq)]
enum EventData {
    StringData(String),
    IntData(i32),
    // 可以在这里添加更多类型
}

type Listener = Arc<dyn Fn(EventData) + Send + Sync>;

struct EventManager {
    listeners: Arc<Mutex<HashMap<String, Vec<Listener>>>>,
}

impl EventManager {
    fn new() -> Self {
        EventManager {
            listeners: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn instance() -> &'static Arc<EventManager> {
        lazy_static! {
            static ref INSTANCE: Arc<EventManager> = Arc::new(EventManager::new());
        }
        &INSTANCE
    }

    fn register_listener(event: &str, listener: Listener) {
        let manager = EventManager::instance();
        let mut listeners = manager.listeners.lock().unwrap();
        listeners.entry(event.to_string()).or_insert(Vec::new()).push(listener);
        ic!("Listener registered for event:");
        ic!(event);
    }

    async fn trigger_event(event: &str, data: EventData) {
        let manager = EventManager::instance();
        let event = event.to_string();  // 克隆 event
        ic!("Triggering event:");
        ic!(&event);
        let listeners = manager.listeners.lock().unwrap();
        if let Some(list) = listeners.get(&event) {
            for listener in list {
                let listener = listener.clone();
                let data = data.clone();
                let event_clone = event.clone();
                ic!("Spawning task for event:");
                ic!(&event_clone);
                task::spawn(async move {
                    ic!("Invoking listener for event:");
                    ic!(&event_clone);
                    listener(data);
                    ic!("Listener invoked for event:");
                    ic!(&event_clone);
                }).await.unwrap();
            }
        } else {
            ic!("No listeners for event:");
            ic!(&event);
        }
    }
}

#[macro_export]
macro_rules! register_listener {
    ($event:expr, $listener:expr) => {
        EventManager::register_listener($event, Arc::new($listener));
    };
}

#[macro_export]
macro_rules! trigger_event {
    ($event:expr, $data:expr) => {
        {
            EventManager::trigger_event($event, $data).await;
        }
    };
}

#[tokio::main]
async fn main() {
    register_listener!("test_event", |data: EventData| {
        ic!("Listener invoked with data:");
        ic!(&data);
        match data {
            EventData::StringData(message) => {
                ic!("Received String:");
                ic!(&message);
                println!("Received String: {}", message);
            },
            EventData::IntData(number) => {
                ic!("Received i32:");
                ic!(number);
                println!("Received i32: {}", number);
            },
            // 可以在这里添加更多类型的处理
        }
    });

    ic!("Before triggering string event");
    trigger_event!("test_event", EventData::StringData("Hello, World!".to_string()));
    ic!("After triggering string event");

    ic!("Before triggering int event");
    trigger_event!("test_event", EventData::IntData(12345));
    ic!("After triggering int event");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Barrier;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_register_listener() {
        let event = "test_register";
        let listener_called = Arc::new(AtomicBool::new(false));

        register_listener!(event, {
            let listener_called = listener_called.clone();
            move |_data: EventData| {
                listener_called.store(true, Ordering::SeqCst);
            }
        });

        EventManager::trigger_event(event, EventData::StringData("Test".to_string())).await;

        assert!(listener_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_trigger_event_string() {
        let event = "test_string";
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = barrier.clone();

        register_listener!(event, move |data: EventData| {
            let barrier_clone = barrier_clone.clone();
            tokio::spawn(async move {
                if let EventData::StringData(ref message) = data {
                    assert_eq!(message, "Hello");
                    barrier_clone.wait().await;
                }
            });
        });

        trigger_event!(event, EventData::StringData("Hello".to_string()));
        barrier.wait().await;
    }

    #[tokio::test]
    async fn test_trigger_event_int() {
        let event = "test_int";
        let barrier = Arc::new(Barrier::new(2));
        let barrier_clone = barrier.clone();

        register_listener!(event, move |data: EventData| {
            let barrier_clone = barrier_clone.clone();
            tokio::spawn(async move {
                if let EventData::IntData(ref number) = data {
                    assert_eq!(*number, 123);
                    barrier_clone.wait().await;
                }
            });
        });

        trigger_event!(event, EventData::IntData(123));
        barrier.wait().await;
    }

    #[tokio::test]
    async fn test_multiple_listeners() {
        let event = "test_multiple";
        let barrier = Arc::new(Barrier::new(3)); // 两个监听器 + main
        let barrier_clone1 = barrier.clone();
        let barrier_clone2 = barrier.clone();

        register_listener!(event, move |data: EventData| {
            let barrier_clone1 = barrier_clone1.clone();
            tokio::spawn(async move {
                if let EventData::StringData(ref message) = data {
                    assert_eq!(message, "Multi");
                    barrier_clone1.wait().await;
                }
            });
        });

        register_listener!(event, move |data: EventData| {
            let barrier_clone2 = barrier_clone2.clone();
            tokio::spawn(async move {
                if let EventData::StringData(ref message) = data {
                    assert_eq!(message, "Multi");
                    barrier_clone2.wait().await;
                }
            });
        });

        trigger_event!(event, EventData::StringData("Multi".to_string()));
        barrier.wait().await;
    }
}
