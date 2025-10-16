use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

type Callback = Arc<dyn Fn(Value) + Send + Sync + 'static>;

pub struct Push {
    pub event: String,
    pub payload: Value,
    pub ref_id: String,
    pub timeout: Duration,
    callbacks: Arc<Mutex<HashMap<String, Callback>>>,
}

impl Push {
    pub fn new(event: String, payload: Value, ref_id: String, timeout: Duration) -> Self {
        Self {
            event,
            payload,
            ref_id,
            timeout,
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn receive<F>(self, status: &str, callback: F) -> Self
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        self.callbacks
            .lock()
            .unwrap()
            .insert(status.to_string(), Arc::new(callback));
        self
    }

    pub fn trigger(&self, status: &str, payload: Value) {
        let opt_callback = {
            let callbacks = self.callbacks.lock().unwrap();
            callbacks.get(status).cloned() // Clone the Arc'd function
        }; // Lock released here

        if let Some(callback) = opt_callback {
            callback(payload);
        }
    }
}
