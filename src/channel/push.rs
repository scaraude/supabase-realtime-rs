use crate::{ChannelEvent, types::Result};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{RealtimeChannel, RealtimeMessage};

type Callback = Arc<dyn Fn(Value) + Send + Sync + 'static>;

pub struct Push {
    pub event: String,
    pub payload: Value,
    pub ref_id: String,
    pub timeout: Duration,
    pub(crate) callbacks: Arc<Mutex<HashMap<String, Callback>>>,
    pub(crate) channel: Arc<RealtimeChannel>,
}

impl Push {
    pub fn new(
        event: String,
        payload: Value,
        ref_id: String,
        timeout: Duration,
        channel: Arc<RealtimeChannel>,
    ) -> Self {
        Self {
            event,
            payload,
            ref_id,
            timeout,
            channel,
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

    pub async fn send(self) -> Result<()> {
        let push_arc = Arc::new(self);

        // Create message to send
        let message = RealtimeMessage::new(
            push_arc.channel.topic().to_string(),
            ChannelEvent::Custom(push_arc.event.clone()),
            push_arc.payload.clone(),
        )
        .with_ref(push_arc.ref_id.clone());

        // Store in pending pushes
        push_arc
            .channel
            .state
            .write()
            .await
            .pending_pushes
            .insert(push_arc.ref_id.clone(), Arc::clone(&push_arc));

        // Send message via client
        push_arc.channel.send_message(message).await?;

        // Spawn timeout task
        let timeout = push_arc.timeout;
        let ref_id = push_arc.ref_id.clone();
        let channel = Arc::clone(&push_arc.channel);

        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            // Check if still pending
            let mut state = channel.state.write().await;
            if let Some(pending_push) = state.pending_pushes.remove(&ref_id) {
                drop(state); // Release lock before triggering
                pending_push.trigger("timeout", Value::Null);
            }
        });

        Ok(())
    }
}
