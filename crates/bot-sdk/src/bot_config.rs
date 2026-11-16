use crate::reconnect_policy::ReconnectPolicy;

#[derive(Clone, Debug)]
pub struct BotConfig {
    /// WebSocket endpoint, e.g. `ws://127.0.0.1:5000/ws` (`wss://` is not supported yet).
    pub url: String,
    /// Bot key issued by the server operator (the server stores only its SHA-256).
    pub bot_key: String,
    /// Informational; shows up in server logs.
    pub client_version: String,
    pub reconnect: ReconnectPolicy,
}

impl BotConfig {
    pub fn new(url: impl Into<String>, bot_key: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            bot_key: bot_key.into(),
            client_version: concat!("mg-bot-sdk/", env!("CARGO_PKG_VERSION")).to_owned(),
            reconnect: ReconnectPolicy::default(),
        }
    }
}
