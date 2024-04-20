
use super::{
    error::{Problem, ProblemLevel},
    inner::{INDENT, CATCH, m_edit, m_edit_read},
};
use std::{
    process,
    sync::Mutex,
};

/// Indent logs an additional level as long as this guard lives.
pub struct LogIndent { 
    linebreaks: Mutex<u32>,
}

/// Indent logs an additional level as long as this guard lives.
#[must_use = "This global data guard will exit immediately if discarded"]
pub fn log_indent() -> LogIndent {
    m_edit(&INDENT, |n| n + 1);
    LogIndent { 
        linebreaks: Mutex::new(0),
    }
}

impl Drop for LogIndent {
    fn drop(&mut self) {
        m_edit(&INDENT, |n| n - 1);
    }
}

impl LogIndent {
    /// End the indentation.
    pub fn end(self) { drop(self); }
    
    /// Put an empty line between repetetions 
    /// of a logged operation.
    pub fn linebreak(&self) {
        if m_edit_read(&&self.linebreaks, |&l| (l + 1, l > 0)) {
            println!();
        }
    }
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
            color!("\n";red "[ EXIT  ] Process failed.";"\n";,);
            process::exit(1);
        }
    }
    
    /// Put an empty line between repetetions 
    /// of a logged operation.
    ///
    /// No-ops if not indented.
    pub fn linebreak(&self) {
        if let Some(indent) = self._indent.as_ref() {
            indent.linebreak();
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