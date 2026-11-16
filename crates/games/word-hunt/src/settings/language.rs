/// The language of the words; it decides the alphabet and the answer list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Russian,
    English,
}

impl Language {
    pub(crate) const fn hash_code(self) -> u8 {
        match self {
            Language::Russian => 0,
            Language::English => 1,
        }
    }

    /// The letter as the game spells it: lowercase, Russian `ё` folded into `е` (word games
    /// and most texts treat them as one letter). `None` for anything outside the alphabet.
    pub fn normalize(self, letter: char) -> Option<char> {
        match self {
            Language::Russian => match letter {
                'ё' | 'Ё' => Some('е'),
                'а'..='я' => Some(letter),
                'А'..='Я' => char::from_u32(u32::from(letter) + 0x20),
                _ => None,
            },
            Language::English => match letter {
                'a'..='z' => Some(letter),
                'A'..='Z' => Some(letter.to_ascii_lowercase()),
                _ => None,
            },
        }
    }

    /// The alphabet in order, as the game spells letters (so without `ё`).
    pub fn alphabet(self) -> impl Iterator<Item = char> {
        let letters = match self {
            Language::Russian => 'а'..='я',
            Language::English => 'a'..='z',
        };
        letters.into_iter()
    }
}
