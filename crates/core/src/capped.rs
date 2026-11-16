use crate::out_of_range::OutOfRange;
use crate::setting_range::SettingRange;

/// A `u32` in `LOW..=HIGH`, valid by construction: the bounds of a numeric setting live in its
/// type, and the UI reads them back through [`Capped::range`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capped<const LOW: u32, const HIGH: u32>(u32);

impl<const LOW: u32, const HIGH: u32> Capped<LOW, HIGH> {
    pub const MIN: u32 = LOW;
    pub const MAX: u32 = HIGH;

    pub const fn new(value: u32) -> Result<Self, OutOfRange> {
        const { assert!(LOW <= HIGH, "empty range") };
        match OutOfRange::check(value, LOW, HIGH) {
            Ok(value) => Ok(Self(value)),
            Err(error) => Err(error),
        }
    }

    /// A value known at compile time; an out-of-range `VALUE` fails the build, never the run.
    pub const fn constant<const VALUE: u32>() -> Self {
        const { assert!(LOW <= VALUE && VALUE <= HIGH, "constant outside its range") };
        Self(VALUE)
    }

    /// The nearest valid value. Only for presenting a value that is still being edited (never for
    /// input that a game will act on: that is rejected, not clamped).
    pub const fn saturating(value: u32) -> Self {
        const { assert!(LOW <= HIGH, "empty range") };
        if value < LOW {
            Self(LOW)
        } else if value > HIGH {
            Self(HIGH)
        } else {
            Self(value)
        }
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    /// Lossless because the bound is checked at compile time.
    pub const fn to_u8(self) -> u8 {
        const { assert!(HIGH <= u8::MAX as u32, "range does not fit a u8") };
        self.0 as u8
    }

    /// Lossless because the bound is checked at compile time.
    pub const fn to_u16(self) -> u16 {
        const { assert!(HIGH <= u16::MAX as u32, "range does not fit a u16") };
        self.0 as u16
    }

    /// The bounds as published to a UI, under the setting's wire field name.
    pub const fn range(field: &'static str) -> SettingRange {
        SettingRange::new(field, LOW, HIGH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Percent = Capped<1, 100>;

    #[test]
    fn accepts_exactly_the_inclusive_bounds() {
        assert_eq!(Percent::new(1).map(Percent::get), Ok(1));
        assert_eq!(Percent::new(100).map(Percent::get), Ok(100));
        assert_eq!(
            Percent::new(0),
            Err(OutOfRange {
                value: 0,
                min: 1,
                max: 100
            })
        );
        assert_eq!(
            Percent::new(u32::MAX),
            Err(OutOfRange {
                value: u32::MAX,
                min: 1,
                max: 100
            })
        );
    }

    #[test]
    fn the_error_reads_like_the_settings_form_shows_it() {
        assert_eq!(
            Percent::new(101).unwrap_err().to_string(),
            "must be between 1 and 100, got 101"
        );
    }

    #[test]
    fn bounds_are_published_from_the_type() {
        assert_eq!((Percent::MIN, Percent::MAX), (1, 100));
        assert_eq!(
            Percent::range("chance"),
            SettingRange::new("chance", 1, 100)
        );
    }

    #[test]
    fn constants_and_saturation_stay_inside() {
        const DEFAULT: Percent = Percent::constant::<50>();
        assert_eq!(DEFAULT.get(), 50);
        assert_eq!(Percent::saturating(0).get(), 1);
        assert_eq!(Percent::saturating(70).get(), 70);
        assert_eq!(Percent::saturating(u32::MAX).get(), 100);
    }

    #[test]
    fn narrows_losslessly() {
        assert_eq!(Capped::<0, 255>::new(255).unwrap().to_u8(), 255);
        assert_eq!(Capped::<0, 65_535>::new(65_535).unwrap().to_u16(), 65_535);
    }

    #[test]
    fn orders_by_value() {
        assert!(Percent::constant::<3>() < Percent::constant::<4>());
    }
}
