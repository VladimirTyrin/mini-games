use common::config::{ConfigManager, FileContentConfigProvider, Validate, YamlConfigSerializer};
use serde::{Deserialize, Serialize};

use super::{
    GameType, ReplayConfig, ServerConfig, SnakeLobbyConfig, TicTacToeLobbyConfig,
};

const CONFIG_FILE_NAME: &str = "mini_games_client_config.yaml";

fn get_config_path() -> String {
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        return exe_dir.join(CONFIG_FILE_NAME).to_string_lossy().into_owned();
    }
    CONFIG_FILE_NAME.to_string()
}

pub fn get_config_manager() -> ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer>
{
    ConfigManager::from_yaml_file(&get_config_path())
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub last_game: Option<GameType>,
    pub snake: SnakeLobbyConfig,
    pub tictactoe: TicTacToeLobbyConfig,
    pub replays: ReplayConfig,
    pub client_id: Option<String>,
    #[serde(default)]
    pub file_association_registered: bool,
}

impl Validate for Config {
    fn validate(&self) -> Result<(), String> {
        self.server.validate()?;
        self.snake.validate()?;
        self.tictactoe.validate()?;
        self.replays.validate()?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                address: Some("http://185.157.212.124:5001".to_string()),
                disconnect_timeout_ms: 200,
            },
            last_game: None,
            snake: SnakeLobbyConfig::default(),
            tictactoe: TicTacToeLobbyConfig::default(),
            replays: ReplayConfig {
                save: true,
                location: "minigamesreplays".to_string(),
            },
            client_id: None,
            file_association_registered: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::config::{ConfigContentProvider, ConfigSerializer, YamlConfigSerializer};
    use common::id_generator::generate_client_id;

    fn get_temp_file_path() -> String {
        use std::env;
        let mut path = env::temp_dir();
        let random_number: u32 = rand::random();
        let file_name = format!("temp_mini_games_client_config_{}.yaml", random_number);
        path.push(file_name);
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_default_config_can_be_serialized_and_deserialized_string() {
        let default_config = Config::default();
        let serializer = YamlConfigSerializer::new();
        let serialize_result = serializer.serialize(&default_config);
        assert!(serialize_result.is_ok());
        let serialized_string = serialize_result.unwrap();
        let deserialize_result = serializer.deserialize(&serialized_string);
        assert!(deserialize_result.is_ok());
        let deserialized_config = deserialize_result.unwrap();
        assert_eq!(default_config, deserialized_config);
    }

    #[test]
    fn test_default_config_can_be_serialized_and_deserialized_file() {
        let default_config = Config::default();
        let serializer = YamlConfigSerializer::new();
        let file_path = get_temp_file_path();
        dbg!(&file_path);
        let content_provider = FileContentConfigProvider::new(file_path);

        let serialize_result = serializer.serialize(&default_config);
        assert!(serialize_result.is_ok());
        let serialized_string = serialize_result.unwrap();
        dbg!(serialized_string.as_str());
        let write_result = content_provider.set_config_content(&serialized_string);
        assert!(write_result.is_ok());

        let read_result = content_provider.get_config_content();
        assert!(read_result.is_ok());
        let read_string = read_result.unwrap().unwrap();

        let deserialize_result = serializer.deserialize(&read_string);
        assert!(deserialize_result.is_ok());
        let deserialized_config = deserialize_result.unwrap();
        assert_eq!(default_config, deserialized_config);
    }

    #[test]
    fn test_default_config_can_be_serialized_and_deserialized_manager() {
        let config = Config {
            client_id: Some(generate_client_id()),
            ..Config::default()
        };
        let serializer = YamlConfigSerializer::new();
        let file_path = get_temp_file_path();
        dbg!(&file_path);
        let content_provider = FileContentConfigProvider::new(file_path);
        let manager = ConfigManager::new(content_provider, serializer);

        let save_result = manager.set_config(&config);
        assert!(save_result.is_ok());

        let get_result = manager.get_config();
        assert!(get_result.is_ok());
        let loaded_config = get_result.unwrap();
        assert_eq!(config, loaded_config);

        let get_again_result = manager.get_config();
        assert!(get_again_result.is_ok());
        let loaded_config_again = get_again_result.unwrap();
        assert_eq!(config, loaded_config_again);
    }

    #[test]
    fn test_config_file_does_not_exist_returns_default_config() {
        let serializer = YamlConfigSerializer::new();

        let file_path = "this_file_does_not_exist.yaml".to_string();
        let content_provider = FileContentConfigProvider::new(file_path);
        let manager: ConfigManager<_, Config, _> = ConfigManager::new(content_provider, serializer);
        let get_result = manager.get_config();
        assert!(get_result.is_ok());
        let loaded_config = get_result.unwrap();
        assert_eq!(Config::default(), loaded_config);
    }

    #[test]
    fn test_invalid_config_cant_be_read() {
        let invalid_config_content = r#"
            server:
              # address is missing
              disconnect_timeout_ms: 200
            lobby:
              max_players: 4
              field_width: 5
              field_height: 5
              wall_collision_mode: WrapAround
        "#;

        let file_path = get_temp_file_path();
        let content_provider = FileContentConfigProvider::new(file_path);
        content_provider
            .set_config_content(invalid_config_content)
            .unwrap();

        let serializer = YamlConfigSerializer::new();
        let manager: ConfigManager<_, Config, _> = ConfigManager::new(content_provider, serializer);
        let get_result = manager.get_config();
        assert!(get_result.is_err());
    }
}
