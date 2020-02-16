
use std::{
    fs::read_to_string,
    path::Path,
    fmt::{self, Display, Formatter},
};
use regex::Regex;
use semver::Version;
use failure::Error;

macro_rules! regexes {
    ($(
        $name:ident = $pattern:expr;
    )*)=>{
        lazy_static::lazy_static! {
            $(
                static ref $name: Regex = Regex::new(AsRef::<str>::as_ref($pattern)).unwrap();
            )*
        }
    };
}

/// An entry in a changelog.
#[derive(Debug, Clone)]
pub struct VersionNote {
    pub version: Version,
    pub body: String,
}

impl Display for VersionNote {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("#### ")?;
        Display::fmt(&self.version, f)?;
        f.write_str("\n\n")?;
        f.write_str(&self.no_trailing_newline())?;
        Ok(())
    }
}

impl VersionNote {
    fn no_trailing_newline(&self) -> &str {
        Some(self.body.len())
            .filter(|&l| l > 0)
            .map(|l| &self.body[..l - 1])
            .unwrap_or("")
    }
}

pub fn read_changelog<P>(path: P) -> Result<Vec<VersionNote>, Error> 
where
    P: AsRef<Path>
{
    let data = read_to_string(&path).map_err(Error::from)?;
    let mut parsed: Vec<(&str, ParsedLine)> = data
        .lines()
        .map(|line| (line, ParsedLine::from(line)))
        .collect();
        
    for &mut (_, ref mut parsed) in &mut parsed {
        parsed.specialize_header();
    }
    
    let mut builder = LogBuilder {
        accum: Vec::new(),
        curr: None,
    };
    
    for &(line, ref parsed) in &parsed {
        if let Some(ref mut curr) = builder.curr.as_mut() {
            match parsed {
                &ParsedLine::Header(pounds, _) => {
                    if pounds >= curr.pounds {
                        builder.finalize_curr();
                    } else {
                        curr.push_line(line);
                    }
                },
                
                &ParsedLine::VersionHeader(pounds, ref version) => {
                    if pounds >= curr.pounds {
                        builder.finalize_curr();
                        builder.curr = Some(PartialEntry {
                            version: version.clone(),
                            pounds,
                            body: "".to_owned(),
                        });
                    } else {
                        curr.push_line(line);
                    }
                },
                
                &ParsedLine::SectionBreak => {
                    builder.finalize_curr();
                },
                
                &ParsedLine::Default => {
                    curr.push_line(line);
                },
            };
        } else {
            match parsed {
                &ParsedLine::VersionHeader(pounds, ref version) => {
                    builder.curr = Some(PartialEntry {
                        version: version.clone(),
                        pounds,
                        body: "".to_owned(),
                    });
                },
                
                _ => (),
            };
        }
    }
    
    builder.finalize_curr();
    Ok(builder.accum)
}

struct LogBuilder {
    accum: Vec<VersionNote>,
    curr: Option<PartialEntry>
}

struct PartialEntry {
    version: Version,
    pounds: i32,
    body: String,
}

enum ParsedLine {
    Header(i32, String),
    VersionHeader(i32, Version),
    SectionBreak,
    Default,
}

impl LogBuilder {
    fn finalize_curr(&mut self) {
        if let Some(mut curr) = self.curr.take() {
            curr.body = curr.body.trim().to_owned();
            curr.body.push('\n');
        
            self.accum.push(VersionNote {
                version: curr.version,
                body: curr.body,
            });
        }
    }
}

impl PartialEntry {
    fn push_line(&mut self, line: &str) {
        self.body.push_str(line);
        self.body.push('\n');
    }
}

impl ParsedLine {
    pub fn specialize_header(&mut self) {
        if let &mut ParsedLine::Header(pounds, ref body) = self {
            if let Ok(version) = Version::parse(body) {
                *self = ParsedLine::VersionHeader(pounds, version);
            }
        }
    }
}

impl<S: AsRef<str>> From<S> for ParsedLine {
    fn from(s: S) -> Self {
        let line = s.as_ref();
        
        regexes! {
            HEADER = r#"^(?P<pounds>#+)\s*(?P<body>.*)$"#;
            SECTION_BREAK = r#"^\-\-\-+\s*$"#;
        }
        
        if let Some(caps) = HEADER.captures(line) {
            let body = caps.name("body").unwrap().as_str().to_owned();
            let pounds = caps.name("pounds").unwrap().as_str().len() as i32;
            
            ParsedLine::Header(pounds, body)
        } else if SECTION_BREAK.is_match(line) {
            ParsedLine::SectionBreak
        } else {
            ParsedLine::Default
        }
    }
}
