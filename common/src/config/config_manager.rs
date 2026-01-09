use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::{
    ConfigContentProvider, ConfigSerializer, FileContentConfigProvider, Validate,
    YamlConfigSerializer,
};

pub struct ConfigManager<TConfigContentProvider, TConfig, TConfigSerializer = YamlConfigSerializer>
where
    TConfigContentProvider: ConfigContentProvider,
    TConfig: Clone + for<'de> Deserialize<'de> + Serialize + Validate + Default,
    TConfigSerializer: ConfigSerializer<TConfig>,
{
    config_serializer: TConfigSerializer,
    config_content_provider: TConfigContentProvider,
    config: Arc<Mutex<Option<TConfig>>>,
}

impl<TConfig> ConfigManager<FileContentConfigProvider, TConfig, YamlConfigSerializer>
where
    TConfig: Clone + for<'de> Deserialize<'de> + Serialize + Validate + Default,
{
    pub fn new(
        config_content_provider: FileContentConfigProvider,
        config_serializer: YamlConfigSerializer,
    ) -> Self {
        Self {
            config: Arc::new(Mutex::new(None)),
            config_content_provider,
            config_serializer,
        }
    }

    pub fn from_yaml_file(file_path: &str) -> Self {
        Self {
            config: Arc::new(Mutex::new(None)),
            config_content_provider: FileContentConfigProvider::new(file_path.to_string()),
            config_serializer: YamlConfigSerializer {},
        }
    }
}

impl<TConfigContentProvider, TConfig, TConfigSerializer>
    ConfigManager<TConfigContentProvider, TConfig, TConfigSerializer>
where
    TConfigContentProvider: ConfigContentProvider,
    TConfig: Clone + for<'de> Deserialize<'de> + Serialize + Validate + Default,
    TConfigSerializer: ConfigSerializer<TConfig>,
{
    pub fn get_config(&self) -> Result<TConfig, String> {
        let mut current = self.config.lock().unwrap();

        if let Some(config) = current.as_ref() {
            return Ok(config.clone());
        }

        let config_data_result = self.config_content_provider.get_config_content()?;
        if let Some(config_data) = config_data_result {
            let config = self.config_serializer.deserialize(&config_data)?;

            config
                .validate()
                .map_err(|e| format!("Config validation error: {}", e))?;

            *current = Some(config.clone());
            return Ok(config);
        }

        Ok(TConfig::default())
    }

    pub fn set_config(&self, config: &TConfig) -> Result<(), String> {
        config
            .validate()
            .map_err(|e| format!("Config validation error: {}", e))?;

        let serialized_config = self.config_serializer.serialize(config)?;

        self.config_content_provider
            .set_config_content(&serialized_config)?;

        let mut current = self.config.lock().unwrap();
        *current = Some(config.clone());
        Ok(())
    }
}
