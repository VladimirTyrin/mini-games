use serde::{Deserialize, Serialize};

pub trait ConfigSerializer<TConfig> {
    fn serialize(&self, config: &TConfig) -> Result<String, String>;
    fn deserialize(&self, content: &str) -> Result<TConfig, String>;
}

pub struct YamlConfigSerializer;

impl Default for YamlConfigSerializer {
    fn default() -> Self {
        Self::new()
    }
}

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
