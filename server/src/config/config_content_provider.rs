use std::io::ErrorKind;

pub trait ConfigContentProvider {
    fn get_config_content(&self) -> Result<Option<String>, String>;
    fn set_config_content(&self, content: &str) -> Result<(), String>;
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
