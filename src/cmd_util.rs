
use crate::cli_util::ResultExt;
use std::{
    io::{Read, Write, BufRead, BufReader, BufWrite},
    path::Path,
    collections::HashMap,
    process::{Command, Stdio, ChildStdout},
    ffi::OsStr,
    mem::replace,
    thread::{self, JoinHandle},
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

pub fn exec_command<I, P, C, O, T>(
    stdin_content: I, workdir: P, cmd: C, stdout_reader: O)
    -> JoinHandle<T>
where 
    I: Read,
    P: AsRef<Path>, 
    C: AsRef<str>, 
    O: FnOnce(ChildStdout) + Send + 'static,
    T: Send + 'static,
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
    
    // spawn subprocess
    let program = program
        .ok_or_else(|| format_err!(
            "cannot find program part of command"))
        .ekill();
    let mut sys_cmd = Command::new(program);
    sys_cmd
        .envs(&vars)
        .args(&args)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .current_dir(&workdir);    
    let sys_cmd_str = format!("{:?}", sys_cmd);
    printbl!("[DEBUG] ", "executing command:\n{}", sys_cmd_str); 
    let subproc = sys_cmd.spawn().ekill();
    let subproc_in = subproc.stdin.take().unwrap();
    let subproc_out = subproc.stdout.take().unwrap();
    
    let sys_cmd_str0 = sys_cmd_str;
    
    // spawn thread to pipe in the stdin content
    thread::Builder::new()
        .name("thread to pipe input into {}", sys_cmd_str)
        .spawn(move || {
            let mut pipe_from = BufReader::new(stdin_content);
            let mut pipe_into = BufWriter::new(subproc_in);
            
            loop {
                let chunk = pipe_from.fill_buf()
                    .map_err(|e| format!("error reading from stdin_content:\n\
                        {}\
                        to subprocess:\
                        {}", e, sys_cmd_str0))
                    .ekill();
                if chunk.len() == 0 {
                    // Quoting [the docs](https://doc.rust-lang.org/std/io/trait.BufRead.html#tymethod.fill_buf)
                    //
                    // > An empty buffer returned indicates that the stream has reached EOF.
                    break;
                }
                let released: usize = match pipe_into.write(chunk) {
                    Ok(n) => n,
                    Err(e) => {
                        printbl!("[DIAGNOSTIC] ", "error writing into stdin:\n\
                            {}\n\
                            to subprocess:\n\
                            {}", e, sys_cmd_str0);
                        break;
                    },
                };
                pipe_from.consume(released);
            }
        });
        
    printbl!("[INFO] ", "executing command:\n{:?}", sys_cmd);
    let status = sys_cmd.status().map_err(Error::from)?;
    if status.success() {
        Ok(())
    } else {
        Err(format_err!("exit code {:?}", status.code()))
    }
    
}
