
//! LEET: Log Emission and Error Termination

/// ANSI color handling.
#[macro_use]
mod color;

/// Error capturing and handling.
pub mod error;

/// Scoped global effects.
mod scope;

/// Indented formatting helpers.
mod indent;

/// Implementation guts.
pub (self) mod inner;

#[doc(opaque)]
pub use scope::{
    LogIndent, log_indent,
    CatchErrors, catch_errors,
};

use log::LevelFilter;


/// Create and install a LEET logger.
pub fn init(mode: LogMode) {
    Logger::new(mode).install();
}

/// Create and install a LEET logger, with
/// configuration from that LOG environment
/// variable.
pub fn init_from_env() {
    let var0: Option<String> = std::env::var("LOG").ok();
    let var1: Option<&str> = var0.as_ref().map(String::as_str);
    match var1 {
        None | Some("default") => {
            init(LogMode::Default);
        },
        Some("verbose") => init(LogMode::Verbose),
        Some("trace") => init(LogMode::Trace),
        val => {
            init(LogMode::Default);
            warn!("invalid LOG value: {:?}", val);
        }
    };
}

/// LEET logger.
#[derive(Clone, Default)]
pub struct Logger {
    mode: LogMode,
}

/// LEET logger verbosity level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LogMode {
    Default,
    Verbose,
    Trace,
}

impl Logger {
    pub fn new(mode: LogMode) -> Self {
        Logger { mode }
    }
    
    /// Install self as the logging backend.
    ///
    /// Panic on failure.
    pub fn install(self) {
        log::set_logger(self.to_static()).unwrap();
        log::set_max_level(self.mode.to_level_filter());
    }
    
    fn to_static(&self) -> &'static Self {
        match self.mode {
            LogMode::Default => &Logger { mode: LogMode::Default },
            LogMode::Verbose => &Logger { mode: LogMode::Verbose },
            LogMode::Trace => &Logger { mode: LogMode::Trace },
        }
    }
}

impl LogMode {
    pub fn to_level_filter(self) -> LevelFilter {
        match self {
            LogMode::Default => LevelFilter::Info,
            LogMode::Verbose => LevelFilter::Debug,
            LogMode::Trace => LevelFilter::Trace,
        }
    }
}

impl Default for LogMode {
    fn default() -> LogMode {
        LogMode::Default
    }
}
