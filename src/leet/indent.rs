
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
