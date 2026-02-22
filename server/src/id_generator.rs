use rand::Rng;

const ADJECTIVES: &[&str] = &[
    "Swift", "Brave", "Clever", "Mighty", "Silent", "Golden", "Wild", "Noble",
    "Fierce", "Gentle", "Quick", "Wise", "Bold", "Proud", "Cunning", "Sly",
];

const NOUNS: &[&str] = &[
    "Falcon", "Bear", "Tiger", "Wolf", "Eagle", "Dragon", "Lion", "Panther",
    "Hawk", "Fox", "Raven", "Cobra", "Shark", "Phoenix", "Lynx", "Viper",
];

pub fn generate_client_id() -> String {
    let mut rng = rand::rng();
    let adjective = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    format!("{} {}", adjective, noun)
}
