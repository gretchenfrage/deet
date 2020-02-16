
use std::{
    str::FromStr,
    fmt::{Debug, Display},
    env,
};
use failure::{Error, format_err};

/// Match on program args.
macro_rules! match_args {
    (match { $($t:tt)* })=>{{
        let args0: Vec<String> = std::env::args().collect();
        let args1: Vec<&str> = args0.iter().map(String::as_str).collect();
        match &args1[1..] { $($t)* }
    }};
}

/// Match on optional env var.
#[allow(unused_macros)]
macro_rules! match_var {
    (match var($key:expr) { $($t:tt)* })=>{{
        let var0: Option<String> = std::env::var($key).ok();
        let var1: Option<&str> = var0.as_ref().map(String::as_str);
        match var1 { $($t)* }
    }};
}

/// Print an abort message, then exit process.
macro_rules! kill {
    ($($t:tt)*)=>{{
        error!($($t)*);
        std::process::exit(1)
    }};
}

pub trait ResultExt<I, E>: Sized {
    /// Unwrap this result, or delegate the error to kill!.
    fn ekill(self) -> I
    where
        E: Display;
}

impl<I, E> ResultExt<I, E> for Result<I, E> {
    /// Unwrap this result, or delegate the error to kill!.
    fn ekill(self) -> I 
    where
        E: Display
    {
        match self {
            Ok(i) => i,
            Err(e) => kill!("{}", e),
        }
    }
}

/// Get and parse env var or abort.
pub fn parse_var<T: FromStr>(name: &str) -> Result<T, Error>
where
    T::Err: Debug
{
    env::var(name)
        .unwrap_or_else(|_|
            kill!("missing required env var {:?}", name))
        .parse::<T>()
        .map_err(|e| format_err!("failed to parse \
            env var {:?}:\n{:#?}", name, e))
}
