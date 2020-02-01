
use std::fmt::{self, Formatter, Display};
use rand::{
    Rng,
    distributions::{Distribution, Standard},
};
    

/// Utility for random ids.
///
/// Integer that Displays as hex. 
///
/// Generatable with `rand`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Hex(pub u32);

impl Distribution<Hex> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Hex {
        Hex(rng.gen())
    }
}

impl Display for Hex {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&format_args!("{:x}", self.0), f)
    }
}