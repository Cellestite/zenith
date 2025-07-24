use derive_more::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, From, Into, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

#[derive(Deref, DerefMut, From, Into, Default, Debug, Clone, Copy, PartialEq, PartialOrd, Add, Sub, Mul, Div, Rem, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign)]
pub struct Degree(f32);

#[derive(Deref, DerefMut, From, Into, Default, Debug, Clone, Copy, PartialEq, PartialOrd,  Add, Sub, Mul, Div, Rem, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign)]
pub struct Radians(f32);

impl From<Degree> for Radians {
    fn from(value: Degree) -> Self {
        Self(value.0 / 180.0 * std::f32::consts::PI)
    }
}

impl From<Radians> for Degree {
    fn from(value: Radians) -> Self {
        Self(value.0 * std::f32::consts::FRAC_1_PI * 180.0)
    }
}