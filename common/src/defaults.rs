use crate::{LobbySettings, WallCollisionMode};

impl LobbySettings {
    pub fn default_settings() -> Self {
        Self {
            field_width: 15,
            field_height: 15,
            wall_collision_mode: WallCollisionMode::WrapAround.into()
        }
    }
}