
use std::{
    sync::Mutex,
    fmt::Arguments,
    ops::Deref,
    process,
};
use lazy_static::lazy_static;
use log::{Log, Record, Level, Metadata, LevelFilter};

macro_rules! colorln {
    (@fg(black))=>{"30"};
    (@fg(red))=>{"31"};
    (@fg(green))=>{"32"};
    (@fg(yellow))=>{"33"};
    (@fg(blue))=>{"34"};
    (@fg(purple))=>{"35"};
    (@fg(cyan))=>{"36"};
    (@fg(white))=>{"37"};
    (@reset)=>{"\x1B[0m"};

    // color part case
    (
        @fstr($accum:expr) $fg:ident $part:expr ; $($tail:tt)*
    )=>{
        colorln!(
            @fstr(concat!(
                $accum,
                "\x1B[",
                colorln!(@fg($fg)),
                "m",
                $part,
                colorln!(@reset)
            ))
            $($tail)*
        )
    };
    
    // uncolored part case
    (
        @fstr($accum:expr) $part:expr ; $($tail:tt)*
    )=>{
        colorln!(
            @fstr(concat!($accum, $part))
            $($tail)*
        )
    };
    
    // base case
    (
        @fstr($fstr:expr) , $($arg:tt)*
    )=>{
        println!($fstr, $($arg)*)
    };
    // base case
    (
        @fstr($fstr:expr) str
    )=>{ $fstr };
    // base case
    (
        @fstr($fstr:expr) format, $($arg:tt)*
    )=>{ format!($fstr, $($arg)*) };
    
    // bootstrapping
    ($($arg:tt)*)=>{ colorln!(@fstr("") $($arg)*) };
}

#[derive(Clone, Default)]
pub struct Logger {
    mode: LogMode,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LogMode {
    Default,
    Verbose,
    Trace,
}

impl Default for LogMode {
    fn default() -> LogMode {
        LogMode::Default
    }
}

pub fn init(mode: LogMode) {
    let logger: &'static Logger = match mode {
        LogMode::Default => &Logger { mode: LogMode::Default },
        LogMode::Verbose => &Logger { mode: LogMode::Verbose },
        LogMode::Trace => &Logger { mode: LogMode::Trace },
    };
    log::set_logger(logger).unwrap();
    log::set_max_level(match mode {
        LogMode::Default => LevelFilter::Info,
        LogMode::Verbose => LevelFilter::Debug,
        LogMode::Trace => LevelFilter::Trace,
    });
}

#[derive(Clone, PartialEq, Debug)]
pub struct PreFormatted {
    display: String,
    debug: String,
    debug_multiline: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct StaticRecord {
    pub body: PreFormatted,
    pub level: Level,
    pub target: String,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
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

impl<'a> From<&'a Record<'a>> for StaticRecord {
    fn from(record: &'a Record<'a>) -> StaticRecord {
        StaticRecord {
            body: record.args().into(),
            level: record.level(),
            target: record.target().to_owned(),
            module_path: record.module_path().map(str::to_owned),
            file: record.file().map(str::to_owned),
            line: record.line(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Catchable {
    Error(StaticRecord),
    Warn(StaticRecord),
}

lazy_static! {
    static ref INDENT: Mutex<u32> = Mutex::new(0);
    static ref CATCH: Mutex<Vec<Vec<Catchable>>> = Mutex::new(Vec::new());
}

pub struct LogIndent { private: () }

#[must_use = "this global data guard will exit immediately if discarded"]
pub fn log_indent() -> LogIndent {
    m_edit(&INDENT, |n| n + 1);
    LogIndent { private: () }
}

impl Drop for LogIndent {
    fn drop(&mut self) {
        m_edit(&INDENT, |n| n - 1);
    }
}

impl LogIndent {
    pub fn end(self) { drop(self); }
}

pub struct CatchErrors {
    indent: Option<LogIndent>,
    handled: bool,
}

#[must_use = "this global data guard will exit immediately if discarded"]
pub fn catch_errors(indent_logs: bool) -> CatchErrors {
    let indent = match indent_logs {
        true => Some(log_indent()),
        false => None,
    };
    
    let mut guard = CATCH.lock().unwrap();
    guard.push(Vec::new());
    
    CatchErrors {
        indent,
        handled: false,
    }
}

impl Drop for CatchErrors {
    fn drop(&mut self) {
        if !self.handled {
            warn!("CatchErrors dropped without handling")
        }
    }
}

impl CatchErrors {
    pub fn get(mut self) -> Vec<Catchable> {
        self.handled = true;
        let mut guard = CATCH.lock().unwrap();
        let vec = guard.pop().unwrap();
        vec
    }
    
    pub fn get_errors(self) -> Vec<StaticRecord> {
        self.get().into_iter()
            .filter_map(|catchable| match catchable {
                Catchable::Warn(_) => None,
                Catchable::Error(record) => Some(record),
            })
            .collect()
    }
    
    pub fn handle(self, pardon_warnings: bool) {
        if pardon_warnings {
            if self.get_errors().len() > 0 {
                process::exit(1);
            }
        } else {
            if self.get().len() > 0 {
                process::exit(1);
            }
        }
    }
}


fn m_read<I, R, O, F>(mutex: &R, func: F) -> O
where
    R: Deref<Target=Mutex<I>>,
    F: FnOnce(&I) -> O
{
    let guard = mutex.lock().unwrap();
    func(&*guard)
}

fn m_edit<I, R, F>(mutex: &R, func: F)
where
    R: Deref<Target=Mutex<I>>,
    F: FnOnce(&I) -> I
{
    let mut guard = mutex.lock().unwrap();
    *guard = func(&*guard)
}

fn m_edit_read<I, R, O, F>(mutex: &R, func: F) -> O
where
    R: Deref<Target=Mutex<I>>,
    F: FnOnce(&I) -> (I, O)
{
    let mut guard = mutex.lock().unwrap();
    let (replace, output) = func(&*guard);
    *guard = replace;
    output
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let level = metadata.level();
        match self.mode {
            LogMode::Default => level <= Level::Info,
            LogMode::Verbose => level <= Level::Debug,
            LogMode::Trace => level <= Level::Trace,
        }
    }
    
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        
        let fstatus: &str = match record.level() {
            Level::Info =>  colorln!(yellow"[ INFO  ]";str),
            Level::Warn =>  colorln!(red   "[ WARN  ]";str),
            Level::Error => colorln!(red   "[ ERROR ]";str),
            Level::Trace => colorln!(cyan  "[ TRACE ]";str),
            Level::Debug => colorln!(blue  "[ INFO  ]";str),
        };
        
        let forigin: String;
        if self.mode == LogMode::Trace {
            forigin = format!("[{}:{}]",
                record.module_path().unwrap_or("?"),
                record.line()
                    .map(|n| format!("{}", n))
                    .unwrap_or(format!("?")));
        } else {
            forigin = format!("");
        }
        let forigin: String = match record.level() {
            Level::Info =>  colorln!(yellow"{}";format, forigin),
            Level::Warn =>  colorln!(red   "{}";format, forigin),
            Level::Error => colorln!(red   "{}";format, forigin),
            Level::Trace => colorln!(cyan  "{}";format, forigin),
            Level::Debug => colorln!(blue  "{}";format, forigin),
        };
        
        let indent = m_read(&INDENT, u32::clone);
        let findent: &str = match indent {
            0 => "",
            1 => "    ",
            2 => "        ",
            3 => "            ",
            _ => "           …",
        };
        
        use indent::{Indent, IndentDisplay};
        
        let display = IndentDisplay {
            indent: Indent {
                base: findent,
                secondary: "          ",
            },
            body: format!("{}{} {}", fstatus, forigin, record.args()),
        };
        println!("{}", display);
        
        if record.level() <= Level::Warn {
            let payload = StaticRecord::from(record);
            let catchable = match record.level() {
                Level::Error => Catchable::Error(payload),
                Level::Warn => Catchable::Warn(payload),
                _ => unreachable!(),
            };

            let mut guard = CATCH.lock().unwrap();
            if let Some(top) = Some(guard.len())
                .filter(|&l| l > 0)
                .map(|l| &mut guard[l - 1])
            {
                top.push(catchable);
            }
        }
    }
    
    fn flush(&self) {}
}

pub mod indent {
    use std::fmt::{self, Display, Formatter, Arguments};

    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
    pub struct Indent<'a> {
        /// Prefix on all lines
        pub base: &'a str,
        /// Prefix on all lines after the first
        pub secondary: &'a str,
    }


    pub struct IndentDisplay<'a> {
        pub indent: Indent<'a>,
        pub body: String,
    }
    
    impl<'a> Display for IndentDisplay<'a> {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            let mut w = IndentWriter {
                indent: self.indent,
                inner: f,
                newline: true,
                firstline: true,
            };
            fmt::write(&mut w, format_args!("{}", self.body))
        }
    }

    pub struct IndentWriter<'a, W: fmt::Write> {
        indent: Indent<'a>,
        inner: W,
        newline: bool,
        firstline: bool,
    }
    
    impl<'a, W: fmt::Write> IndentWriter<'a, W> {
        fn maybe_indent(&mut self) -> fmt::Result {
            if self.newline {
                self.inner.write_str(self.indent.base)?;
                if self.firstline {
                    self.firstline = false;
                } else {
                    self.inner.write_str(self.indent.secondary)?;
                }
                self.newline = false;
            }
            Ok(())
        }
    }
    
    impl<'a, W: fmt::Write> fmt::Write for IndentWriter<'a, W> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let mut first_line = true;
        
            for line in s.split('\n') {
                if first_line {
                    self.maybe_indent()?;
                    self.inner.write_str(line)?;
                    first_line = false;
                } else {
                    self.inner.write_char('\n')?;
                    self.newline = true;
                    self.maybe_indent()?;
                    self.inner.write_str(line)?;
                }
            }
            Ok(())
        }
    
        fn write_char(&mut self, c: char) -> fmt::Result {
            self.inner.write_char(c)?;
            if c == '\n' {
                self.newline = true;
            }
            Ok(())
        }
    
        fn write_fmt(&mut self, args: Arguments) -> fmt::Result {
            fmt::write(self, args)
        }
    }
}


/*
[ INFO  ] 
[ WARN  ] 
[ ERROR ]
[ INFO  ]
[ TRACE ]
*/