use std::time::Duration;

pub struct ClientSettings {
    pub server_address: String,
    pub disconnect_timeout: Duration,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            server_address: "http://[::1]:5001".to_string(),
            disconnect_timeout: Duration::from_millis(200),
        }
    }
}
