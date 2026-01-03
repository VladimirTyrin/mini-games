pub(crate) use common::config::{ConfigManager, FileContentConfigProvider, Validate, YamlConfigSerializer};
use common::WallCollisionMode;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "snake_game_client_config.yaml";

pub fn get_config_manager() -> ConfigManager<FileContentConfigProvider, Config, YamlConfigSerializer> {
    ConfigManager::from_yaml_file(CONFIG_FILE)
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub lobby: LobbyConfig,
    pub client_id: Option<String>,
}

impl Validate for Config {
    fn validate(&self) -> Result<(), String> {
        self.server.validate()?;
        self.lobby.validate()?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub address: String,
    pub disconnect_timeout_ms: u32,
}

impl Validate for ServerConfig {
    fn validate(&self) -> Result<(), String> {
        if self.address.is_empty() {
            return Err("server address must not be empty".to_string());
        }
        if self.disconnect_timeout_ms == 0 {
            return Err("disconnect_timeout_ms must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct LobbyConfig {
    pub max_players: u32,
    pub field_width: u32,
    pub field_height: u32,
    pub wall_collision_mode: WallCollisionMode,
    pub tick_interval_ms: u32,
    pub max_food_count: u32,
    pub food_spawn_probability: f32,
}

impl Validate for LobbyConfig {
    fn validate(&self) -> Result<(), String> {
        if self.max_players == 0 {
            return Err("max_players must be greater than 0".to_string());
        }
        if self.max_players > 8 {
            return Err("max_players must not exceed 8".to_string());
        }
        if self.field_width < 5 || self.field_height < 5 {
            return Err("field dimensions must be at least 5x5".to_string());
        }
        if self.field_width > 25 || self.field_height > 25 {
            return Err("field dimensions must not exceed 25x25".to_string());
        }
        if self.tick_interval_ms < 50 {
            return Err("tick_interval_ms must be at least 50".to_string());
        }
        if self.tick_interval_ms > 1000 {
            return Err("tick_interval_ms must not exceed 1000".to_string());
        }
        if self.max_food_count < 1 {
            return Err("max_food_count must be at least 1".to_string());
        }
        if self.food_spawn_probability <= 0.0 || self.food_spawn_probability > 1.0 {
            return Err("food_spawn_probability must be greater than 0 and at most 1".to_string());
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                address: "http://185.157.212.124:5001".to_string(),
                disconnect_timeout_ms: 200,
            },
            lobby: LobbyConfig {
                max_players: 4,
                field_width: 15,
                field_height: 15,
                wall_collision_mode: WallCollisionMode::WrapAround,
                tick_interval_ms: 200,
                max_food_count: 1,
                food_spawn_probability: 1.0,
            },
            client_id: None,
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
        let file_name = format!("temp_snake_game_client_config_{}.yaml", random_number);
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
        let config = Config { client_id: Some(generate_client_id()), ..Config::default() };
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
        content_provider.set_config_content(invalid_config_content).unwrap();

        let serializer = YamlConfigSerializer::new();
        let manager: ConfigManager<_, Config, _> = ConfigManager::new(content_provider, serializer);
        let get_result = manager.get_config();
        assert!(get_result.is_err());
    }

}