/// Every spelling of a game id the scaffold needs.
#[derive(Debug, Clone)]
pub(crate) struct GameNames {
    /// `connect_four`: game id, proto field, Rust module.
    pub(crate) snake: String,
    /// `connect-four`: crate and web folder.
    pub(crate) kebab: String,
    /// `ConnectFour`: Rust type, proto message prefix, prost oneof variant.
    pub(crate) pascal: String,
    /// `connectFour`: protobuf-es oneof case.
    pub(crate) camel: String,
    /// `Connect Four`: shown to players.
    pub(crate) title: String,
}

impl GameNames {
    pub(crate) fn new(id: &str, title: Option<String>) -> Result<Self, String> {
        let valid = id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !id.contains("__")
            && !id.ends_with('_');
        if !valid {
            return Err(format!("`{id}` is not a snake_case id like `connect_four`"));
        }
        let parts: Vec<&str> = id.split('_').collect();
        let capitalize = |part: &str| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        };
        let pascal: String = parts.iter().map(|part| capitalize(part)).collect();
        let camel = parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    (*part).to_owned()
                } else {
                    capitalize(part)
                }
            })
            .collect();
        let title = title.unwrap_or_else(|| {
            parts
                .iter()
                .map(|part| capitalize(part))
                .collect::<Vec<_>>()
                .join(" ")
        });
        Ok(Self {
            snake: id.to_owned(),
            kebab: id.replace('_', "-"),
            pascal,
            camel,
            title,
        })
    }

    pub(crate) fn render(&self, template: &str) -> String {
        template
            .replace("{{snake_crate}}", &self.snake)
            .replace("{{snake}}", &self.snake)
            .replace("{{kebab}}", &self.kebab)
            .replace("{{PascalField}}", &self.pascal)
            .replace("{{Pascal}}", &self.pascal)
            .replace("{{camel}}", &self.camel)
            .replace("{{Title}}", &self.title)
            .replace("{{SCREAMING}}", &self.snake.to_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::GameNames;

    #[test]
    fn derives_every_spelling() {
        let names = GameNames::new("connect_four", None).unwrap();
        assert_eq!(names.kebab, "connect-four");
        assert_eq!(names.pascal, "ConnectFour");
        assert_eq!(names.camel, "connectFour");
        assert_eq!(names.title, "Connect Four");
    }

    #[test]
    fn rejects_bad_ids() {
        for id in ["", "Connect", "connect-four", "4inarow", "a__b", "a_"] {
            assert!(GameNames::new(id, None).is_err(), "{id}");
        }
    }
}
