
use std::{
    fmt::{self, Formatter, Debug, Display, Write, Arguments},
    error::Error,
    backtrace::Backtrace,
    sync::Arc,
};
use log::{Record, Level};

/// Error or Warn produced captured from a log line.
#[derive(Debug, Clone)]
pub struct Problem {
    level: ProblemLevel,
    record: OwnedRecord,
    backtrace: Option<Arc<Backtrace>>,
}

/// Whether a Problem is level Error or Warn.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ProblemLevel {
    Warn,
    Error,
}

impl Problem {
    pub fn new(
        level: ProblemLevel, 
        record: OwnedRecord, 
        backtrace: Option<Arc<Backtrace>>
    ) -> Self {
        Problem {
            level,
            record,
            backtrace,
        }
    }
    
    pub fn level(&self) -> ProblemLevel {
        self.level
    }
    
    pub fn record(&self) -> &OwnedRecord {
        &self.record
    }
    
    pub fn backtrace(&self) -> Option<&Backtrace> {
        self.backtrace.as_ref().map(Arc::as_ref)
    }
    
    pub fn backtrace_arc(&self) -> Option<&Arc<Backtrace>> {
        self.backtrace.as_ref()
    }
    
    pub fn into_record(self) -> OwnedRecord {
        self.record
    }
}

impl Display for Problem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}[{}:{}] {}", 
            self.level, 
            self.record.file.as_ref().map(String::as_str).unwrap_or("?"),
            self.record.line.map(|n| n.to_string()).unwrap_or("?".to_string()),
            self.record.body))?;
        
        if let Some(backtrace) = self.backtrace.as_ref() {
            f.write_char('\n')?;
            Display::fmt(backtrace, f)?;
        }
        
        Ok(())
    }
}

impl Display for ProblemLevel {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ProblemLevel::Error => f.write_str("[ ERROR ]"),
            ProblemLevel::Warn => f.write_str("[ WARN  ]"),
        }
    }
}

impl Error for Problem {
    fn backtrace(&self) -> Option<&Backtrace> {
        self.backtrace()
    }
}

/// Owned equivalent to log::Record. 
#[derive(Clone, PartialEq, Debug)]
pub struct OwnedRecord {
    pub body: PreFormatted,
    pub level: Level,
    pub target: String,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl<'a> From<&'a Record<'a>> for OwnedRecord {
    fn from(record: &'a Record<'a>) -> OwnedRecord {
        OwnedRecord {
            body: record.args().into(),
            level: record.level(),
            target: record.target().to_owned(),
            module_path: record.module_path().map(str::to_owned),
            file: record.file().map(str::to_owned),
            line: record.line(),
        }
    }
}

/// Owned equivalent to std::fmt::Arguments.
#[derive(Clone, PartialEq)]
pub struct PreFormatted {
    display: String,
    debug: String,
    debug_multiline: String,
}

impl Debug for PreFormatted {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str(&self.debug_multiline)
        } else {
            f.write_str(&self.debug)
        }
    }
}

impl Display for PreFormatted {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.display)
    }
}

impl<'a> From<&'a Arguments<'a>> for PreFormatted {
    fn from(args: &'a Arguments<'a>) -> PreFormatted {
        PreFormatted {
            display: format!("{}", args),
            debug: format!("{:?}", args),
            debug_multiline: format!("{:#?}", args),
        }
    }
}

