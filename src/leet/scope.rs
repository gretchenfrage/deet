
use super::{
    error::{Problem, ProblemLevel},
    inner::{INDENT, CATCH, m_edit},
};
use std::process;

/// Indent logs an additional level as long as this guard lives.
pub struct LogIndent { _private: () }

/// Indent logs an additional level as long as this guard lives.
#[must_use = "This global data guard will exit immediately if discarded"]
pub fn log_indent() -> LogIndent {
    m_edit(&INDENT, |n| n + 1);
    LogIndent { _private: () }
}

impl Drop for LogIndent {
    fn drop(&mut self) {
        m_edit(&INDENT, |n| n - 1);
    }
}

impl LogIndent {
    /// End the indentation.
    pub fn end(self) { drop(self); }
}


/// Catch errors on the global scope for the lifetime of this guard.
pub struct CatchErrors {
    _indent: Option<LogIndent>,
    handled: bool,
}

/// Catch errors on the global scope for the lifetime of this guard.
#[must_use = "This global data guard will exit immediately if discarded"]
pub fn catch_errors(indent_logs: bool) -> CatchErrors {
    let indent = match indent_logs {
        true => Some(log_indent()),
        false => None,
    };
    
    let mut guard = CATCH.lock().unwrap();
    guard.push(Vec::new());
    
    CatchErrors {
        _indent: indent,
        handled: false,
    }
}

impl CatchErrors {
    /// End the catch scope, retriving all errors
    /// that occured.
    pub fn get(mut self) -> Vec<Problem> {
        self.handled = true;
        let mut guard = CATCH.lock().unwrap();
        let vec = guard.pop().unwrap();
        vec
    }
    
    /// If any non-pardoned errors occured, exit the 
    /// process.
    pub fn handle(self, pardon_warnings: bool) {
        let mut problems = self.get();
        if pardon_warnings {
            problems.retain(|p| p.level() == ProblemLevel::Error);
        }
        if problems.len() > 0 {
            process::exit(1);
        }
    }
}

impl Drop for CatchErrors {
    fn drop(&mut self) {
        if !self.handled {
            warn!("CatchErrors dropped without handling")
        }
    }
}