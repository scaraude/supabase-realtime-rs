/// Phoenix protocol event strings (magic strings layer)
pub mod phoenix_events {
    pub const CLOSE: &str = "phx_close";
    pub const ERROR: &str = "phx_error";
    pub const JOIN: &str = "phx_join";
    pub const REPLY: &str = "phx_reply";
    pub const LEAVE: &str = "phx_leave";
    pub const HEARTBEAT: &str = "heartbeat";
}

/// Phoenix protocol topics
pub const PHOENIX_TOPIC: &str = "phoenix";

/// Channel event strings (magic strings layer)
pub mod channel_events {
    pub const ACCESS_TOKEN: &str = "access_token";
    pub const POSTGRES_CHANGES: &str = "postgres_changes";
    pub const BROADCAST: &str = "broadcast";
    pub const PRESENCE: &str = "presence";
    pub const PRESENCE_STATE: &str = "presence_state";
    pub const PRESENCE_DIFF: &str = "presence_diff";
}

/// WebSocket transport
pub const TRANSPORT_WEBSOCKET: &str = "websocket";

/// Protocol version
pub const VSN: &str = "1.0.0";

/// Default timeout (milliseconds)
pub const DEFAULT_TIMEOUT: u64 = 10000;

/// Default heartbeat interval (milliseconds)
pub const HEARTBEAT_INTERVAL: u64 = 25000;

/// Default reconnect intervals (milliseconds)
pub const RECONNECT_INTERVALS: [u64; 4] = [1000, 2000, 5000, 10000];
pub const DEFAULT_RECONNECT_FALLBACK: u64 = 10000;

/// Max push buffer size
pub const MAX_PUSH_BUFFER_SIZE: usize = 1000;

/// WebSocket close codes
pub const WS_CLOSE_NORMAL: u16 = 1000;
