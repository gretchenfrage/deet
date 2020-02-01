
use std::{
    str::FromStr,
    fmt::{self, Debug, Display},
    env,
    io::{stdout, Write},
};
use failure::{Error, format_err};
use unicode_segmentation::UnicodeSegmentation;

/// Match on program args.
macro_rules! match_args {
    (match { $($t:tt)* })=>{{
        let args0: Vec<String> = std::env::args().collect();
        let args1: Vec<&str> = args0.iter().map(String::as_str).collect();
        match &args1[1..] { $($t)* }
    }};
}

/// Print an abort message, then exit process.
macro_rules! kill {
    ($($t:tt)*)=>{{
        $crate::cli_util::printblock("[ABORT] ", format_args!($($t)*));
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

/// Print formatting helper.
pub fn printblock(tag: &str, block: fmt::Arguments) {
    let block = format!("{}", block);
    //let tag_len = tag.chars().count();
    let tag_len = tag.graphemes(true).count();
    let stdout = stdout();
    let mut stdout_write = stdout.lock();
    for (i, line) in block.lines().enumerate() {
        if i == 0 {
            stdout_write.write_fmt(format_args!(
                "{}{}\n", tag, line))
                .expect("stdout failure");
        } else {
            for _ in 0..tag_len {
                stdout_write.write(&[b' '])
                    .expect("stdout failure");
            }
            stdout_write.write_fmt(format_args!(
                "{}\n", line))
                .expect("stdout failure");
        }
    }
}

/// Delegate to printblock.
macro_rules! printbl {
    ($tag:expr, $($t:tt)*)=>{ 
        $crate::cli_util::printblock($tag, format_args!($($t)*)) 
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
