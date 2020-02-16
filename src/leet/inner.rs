
use super::{
    {Logger, LogMode},
    error::{
        OwnedRecord, 
        Problem, 
        ProblemLevel,
    },
    indent::{Indent, IndentDisplay},
};
use std::{
    io::{stdout, Write},
    sync::Mutex,
    ops::Deref,
    backtrace::Backtrace,
    sync::Arc,
};
use log::{Record, Log, Metadata, Level};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INDENT: Mutex<u32> = Mutex::new(0);
    pub static ref CATCH: Mutex<Vec<Vec<Problem>>> = Mutex::new(Vec::new());
}

#[allow(dead_code)]
pub fn m_read<I, R, O, F>(mutex: &R, func: F) -> O
where
    R: Deref<Target=Mutex<I>>,
    F: FnOnce(&I) -> O
{
    let guard = mutex.lock().unwrap();
    func(&*guard)
}

#[allow(dead_code)]
pub fn m_edit<I, R, F>(mutex: &R, func: F)
where
    R: Deref<Target=Mutex<I>>,
    F: FnOnce(&I) -> I
{
    let mut guard = mutex.lock().unwrap();
    *guard = func(&*guard)
}

#[allow(dead_code)]
pub fn m_edit_read<I, R, O, F>(mutex: &R, func: F) -> O
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
            Level::Info =>  color!(yellow"[ INFO  ]";str),
            Level::Warn =>  color!(red   "[ WARN  ]";str),
            Level::Error => color!(red   "[ ERROR ]";str),
            Level::Trace => color!(cyan  "[ TRACE ]";str),
            Level::Debug => color!(blue  "[ INFO  ]";str),
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
            Level::Info =>  color!(yellow"{}";format, forigin),
            Level::Warn =>  color!(red   "{}";format, forigin),
            Level::Error => color!(red   "{}";format, forigin),
            Level::Trace => color!(cyan  "{}";format, forigin),
            Level::Debug => color!(blue  "{}";format, forigin),
        };
        
        let indent = m_read(&INDENT, u32::clone);
        let findent: &str = match indent {
            0 => "",
            1 => "    ",
            2 => "        ",
            3 => "            ",
            _ => "           …",
        };
        
        let display = IndentDisplay {
            indent: Indent {
                base: findent,
                secondary: "          ",
            },
            body: format!("{}{} {}", fstatus, forigin, record.args()),
        };
        println!("{}", display);
        
        if record.level() <= Level::Warn {
            let mut guard = CATCH.lock().unwrap();
            if let Some(top) = Some(guard.len())
                .filter(|&l| l > 0)
                .map(|l| &mut guard[l - 1])
            {
                let problem = Problem::new(
                    match record.level() {
                        Level::Error => ProblemLevel::Error,
                        Level::Warn => ProblemLevel::Warn,
                        _ => unreachable!(),
                    },
                    OwnedRecord::from(record),
                    Some(Arc::new(Backtrace::capture())),
                );
                top.push(problem);
            }
        }
    }
    
    fn flush(&self) {
        stdout().flush().expect("failed to flush stdout");
    }
}
