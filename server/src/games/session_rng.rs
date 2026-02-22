use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub struct SessionRng {
    rng: StdRng,
    seed: u64,
}

impl SessionRng {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    pub fn from_random() -> Self {
        let seed: u64 = rand::rng().random();
        Self::new(seed)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn random<T>(&mut self) -> T
    where
        rand::distr::StandardUniform: rand::distr::Distribution<T>,
    {
        self.rng.random()
    }

    pub fn random_range<T, R>(&mut self, range: R) -> T
    where
        T: rand::distr::uniform::SampleUniform,
        R: rand::distr::uniform::SampleRange<T>,
    {
        self.rng.random_range(range)
    }

    pub fn random_bool(&mut self) -> bool {
        self.rng.random()
    }
}
