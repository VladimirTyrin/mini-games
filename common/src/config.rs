use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

pub trait ConfigSerializer<TConfig> {
    fn serialize(&self, config: &TConfig) -> Result<String, String>;
    fn deserialize(&self, content: &str) -> Result<TConfig, String>;
}

pub trait ConfigContentProvider {
    fn get_config_content(&self) -> Result<Option<String>, String>;
    fn set_config_content(&self, content: &str) -> Result<(), String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

pub struct YamlConfigSerializer;

impl YamlConfigSerializer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<TConfig> ConfigSerializer<TConfig> for YamlConfigSerializer
where
    TConfig: for<'de> Deserialize<'de> + Serialize,
{
    fn serialize(&self, config: &TConfig) -> Result<String, String> {
        serde_yaml_ng::to_string(config).map_err(|e| format!("Failed to serialize config: {}", e))
    }

    fn deserialize(&self, content: &str) -> Result<TConfig, String> {
        serde_yaml_ng::from_str(content).map_err(|e| format!("Failed to deserialize config: {}", e))
    }
}

pub struct FileContentConfigProvider {
    file_path: String,
}

impl FileContentConfigProvider {
    pub fn new(file_path: String) -> Self {
        Self { file_path }
    }
}

impl ConfigContentProvider for FileContentConfigProvider {
    fn get_config_content(&self) -> Result<Option<String>, String> {
        let read_result = std::fs::read_to_string(self.file_path.as_str());

        match read_result {
            Ok(content) => { Ok(Some(content))}
            Err(err) => {
                match err.kind() {
                    ErrorKind::NotFound => { Ok(None) }
                    _ => { Err(format!("Failed to read config file: {}", err)) }
                }
            }
        }
    }

    fn set_config_content(&self, content: &str) -> Result<(), String> {
        std::fs::write(self.file_path.as_str(), content)
            .map_err(|e| format!("Failed to write config file: {}", e))
    }
}

pub struct ConfigManager<TConfigContentProvider, TConfig, TConfigSerializer = YamlConfigSerializer>
where
    TConfigContentProvider: ConfigContentProvider,
    TConfig: Clone + for<'de> Deserialize<'de> + Serialize,
    TConfigSerializer: ConfigSerializer<TConfig>,
{
    config_serializer: TConfigSerializer,
    config_content_provider: TConfigContentProvider,
    config: Arc<Mutex<Option<TConfig>>>,
}

impl<TConfig> ConfigManager<FileContentConfigProvider, TConfig, YamlConfigSerializer>
where
    TConfig: Clone + for<'de> Deserialize<'de> + Serialize,
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
            config_content_provider: FileContentConfigProvider { file_path: file_path.to_string() },
            config_serializer: YamlConfigSerializer {},
        }
    }
}
impl<TConfigContentProvider, TConfig, TConfigSerializer> ConfigManager<TConfigContentProvider, TConfig, TConfigSerializer>
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

            config.validate().map_err(|e| format!("Config validation error: {}", e))?;

            *current = Some(config.clone());
            return Ok(config)
        }

        Ok(TConfig::default())
    }

    pub fn set_config(&self, config: &TConfig) -> Result<(), String> {
        config.validate().map_err(|e| format!("Config validation error: {}", e))?;

        let serialized_config = self.config_serializer.serialize(config)?;

        self.config_content_provider.set_config_content(&serialized_config)?;

        let mut current = self.config.lock().unwrap();
        *current = Some(config.clone());
        Ok(())
    }
}




