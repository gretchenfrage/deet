
//! Display helpers.

use std::fmt::{self, Formatter, Display};

/// Sequence of `Display` which `Display`s each on own line.
#[derive(Debug, Clone)]
pub struct Lines<I>(pub I)
where
    I: Clone + IntoIterator,
    I::Item: Display;
    
impl<I> Display for Lines<I>
where
    I: Clone + IntoIterator,
    I::Item: Display
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut buf = String::new();
        for elem in self.0.clone() {
            buf.push_str(&format!("{}\n", elem));
        }
        if buf.len() > 0 {
            buf.pop();
        }
        f.write_str(&buf)
    }
}

/// Sequence of `Display` which `Display`s each on own
/// line, after mapping through a function.
#[derive(Debug, Clone)]
pub struct LinesView<'a, I, F, A, B>(pub I, pub F)
where
    I: Clone + IntoIterator<Item=&'a A>,
    F: Fn(&A) -> &B,
    A: 'a,
    B: Display;
    
impl<'a, I, F, A, B> Display for LinesView<'a, I, F, A, B>
where
    I: Clone + IntoIterator<Item=&'a A>,
    F: Fn(&A) -> &B,
    A: 'a,
    B: Display
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut buf = String::new();
        for elem in self.0.clone() {
            buf.push_str(&format!("{}\n", (self.1)(elem)));
        }
        if buf.len() > 0 {
            buf.pop();
        }
        f.write_str(&buf)
    }
}