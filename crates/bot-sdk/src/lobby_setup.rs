use mg_protocol::proto;

/// A lobby a bot hosts.
#[derive(Clone, Debug)]
pub struct LobbySetup {
    pub name: String,
    pub settings: proto::GameSettings,
    /// 0 lets the server pick the game's maximum.
    pub max_seats: u32,
    /// Server-side bots seated after the host, in this order.
    pub built_in_bots: Vec<proto::BotKind>,
}
