
use crate::cli_util::ResultExt;
use std::{
    path::Path,
    collections::HashMap,
    process::Command,
    ffi::OsStr,
    mem::replace,
};
use failure::{Error, format_err};
use regex::Regex;

/// Run a subcommand
macro_rules! exec {
    ($workdir:expr, $($t:tt)*)=>{
        $crate::cmd_util::exec_command(
            $workdir, format!($($t)*)).ekill();
    };
}

/// Split a string into words, with awareness of quotes 
/// and escaping.
fn smart_split<S: AsRef<str>>(input: S) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    
    let mut esc_mode = false;
    let mut quote_mode = false;
    let mut curr_buff = String::new();
    
    for c in input.as_ref().chars() {
        if c == '\\' && !esc_mode {
            esc_mode = true;
        } else if esc_mode {
            curr_buff.push(c);
            esc_mode = false;
        } else if c == '\"' {
            quote_mode = !quote_mode;
        } else if c.is_ascii_whitespace() && !quote_mode {
            if curr_buff.len() > 0 {
                parts.push(replace(&mut curr_buff, String::new()));
            }
        } else {
            curr_buff.push(c);
        }
    }
    
    if curr_buff.len() > 0 { parts.push(curr_buff); }
    
    parts
}

pub fn exec_command<P, C>(
    workdir: P, cmd: C) -> Result<(), Error>
where 
    P: AsRef<Path>, 
    C: AsRef<str>, 
{
    // parse command
    let evar_pat = r#"^(?P<key>[^=]+)=(?P<val>[^=]+)$"#;
    let evar_pat = Regex::new(evar_pat).unwrap();
    
    let mut vars: HashMap<&OsStr, &OsStr> = HashMap::new();
    let mut program: Option<&OsStr> = None;
    let mut args: Vec<&OsStr> = Vec::new();
    
    let parts = smart_split(cmd);
    for part in &parts {
        if program.is_none() {
            if let Some(evar_cap) = evar_pat.captures(part) {
                let key: &OsStr = evar_cap.name("key").unwrap()
                    .as_str().as_ref();
                let val: &OsStr = evar_cap.name("val").unwrap()
                    .as_str().as_ref();
                vars.insert(key, val);
            } else {
                program = Some(part.as_ref());
            }
        } else {
            args.push(part.as_ref());
        }
    }
    
    // execute
    let program = program
        .ok_or_else(|| format_err!(
            "cannot find program part of command"))
        .ekill();

    let status = Command::new(program)
        .envs(&vars)
        .args(&args)
        .current_dir(&workdir)
        .status()
        .map_err(Error::from)?;
    if status.success() {
        Ok(())
    } else {
        Err(format_err!("exit code {:?}", status.code()))
    }
    
}
