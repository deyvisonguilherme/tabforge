use num_rational::Rational32;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Exact musical duration represented as a rational fraction of a whole note (1/1).
///
/// Examples:
/// - 1/1 = Whole Note (Semibreve)
/// - 1/2 = Half Note (Mínima)
/// - 1/4 = Quarter Note (Semínima)
/// - 1/8 = Eighth Note (Colcheia)
/// - 1/16 = Sixteenth Note (Semicolcheia)
/// - 1/32 = Thirty-second Note (Fusa)
/// - 1/64 = Sixty-fourth Note (Semifusa)
/// - 1/6 = Quarter Triplet
/// - 1/12 = Eighth Triplet
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Duration(pub Rational32);

impl Duration {
    pub const ZERO: Self = Self(Rational32::new_raw(0, 1));
    pub const WHOLE: Self = Self(Rational32::new_raw(1, 1));
    pub const HALF: Self = Self(Rational32::new_raw(1, 2));
    pub const QUARTER: Self = Self(Rational32::new_raw(1, 4));
    pub const EIGHTH: Self = Self(Rational32::new_raw(1, 8));
    pub const SIXTEENTH: Self = Self(Rational32::new_raw(1, 16));
    pub const THIRTY_SECOND: Self = Self(Rational32::new_raw(1, 32));
    pub const SIXTY_FOURTH: Self = Self(Rational32::new_raw(1, 64));

    pub fn new(numerator: i32, denominator: i32) -> Self {
        Self(Rational32::new(numerator, denominator))
    }

    pub fn as_rational(&self) -> Rational32 {
        self.0
    }

    pub fn as_f64(&self) -> f64 {
        *self.0.numer() as f64 / *self.0.denom() as f64
    }

    /// Add a dot to the duration (1.5x length)
    pub fn dotted(self) -> Self {
        Self(self.0 * Rational32::new(3, 2))
    }

    /// Convert to triplet (2/3 length of regular note)
    pub fn triplet(self) -> Self {
        Self(self.0 * Rational32::new(2, 3))
    }
}

impl Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.0.numer(), self.0.denom())
    }
}
