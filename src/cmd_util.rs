
use crate::cli_util::ResultExt;
use std::{
    io::{Read, Write, BufRead, BufReader, BufWriter},
    path::Path,
    collections::HashMap,
    process::{Command, Child, Stdio, ChildStdout},
    ffi::OsStr,
    mem::replace,
    thread,
};
use failure::{Error, format_err};
use regex::Regex;

/// Subprocesss DSL.
macro_rules! exec {
    // starting with command
    ( [$($c:tt)*] $($t:tt)* )=>{
        exec!(@recurse(
            last=cmd,
            // feed input with an empty cursor
            exec!(@cmd(
                std::io::Cursor::new([]),
                $($c)*
            )),
            $($t)*
        ))
    };
    // starting with value
    ( ($v:expr) $($t:tt)* )=>{
        exec!(@recurse(
            last=fnc,
            $v,
            $($t)*
        ))
    };
    
    // pipe through function
    (@recurse(
        last=$last:ident,
        $curr:expr,
        | ($f:ident) $($t:tt)*
    ))=>{{
        let curr = exec!(@outjoin(
            last=$last,
            $curr
        ));
        exec!(@recurse(
            last=fnc,
            $f(curr),
            $($t)*
        ))
    }};
    // pipe through closure
    (@recurse(
        last=$last:ident,
        $curr:expr,
        | ($f:expr) $($t:tt)*
    ))=>{{
        let curr = exec!(@outjoin(
            last=$last,
            $curr
        ));
        exec!(@recurse(
            last=fnc,
            ($f)(curr),
            $($t)*
        ))
    }};
    // pipe through command
    (@recurse(
        last=$last:ident,
        $curr:expr,
        | [$($c:tt)*] $($t:tt)*
    ))=>{{
        let (subproc, subproc_stdout) = $curr;
        std::thread::spawn(move || {
            $crate::cmd_util::pjoin(subproc).ekill();
        });
        exec!(@recurse(
            last=cmd,
            exec!(@cmd(
                subproc_stdout,
                $($c)*
            )),
            $($t)*    
        ))
    }};
    
    // finish with function(/closure)
    (@recurse(
        last=fnc,
        $curr:expr,
    ))=>{ $curr };
    
    // finish with command
    (@recurse(
        last=cmd,
        $curr:expr,
    ))=>{{
        //$crate::cmd_util::printout($curr);
        
        let (subproc, subproc_stdout) = $curr;
        $crate::cmd_util::printout(subproc_stdout);
        $crate::cmd_util::pjoin(subproc).ekill();
        
    }};
    
    (@outjoin(
        last=fnc,
        $curr:expr
    ))=>{ $curr };
    
    (@outjoin(
        last=cmd,
        $curr:expr
    ))=>{{
        let (subproc, subproc_stdout) = $curr;
        $crate::cmd_util::pjoin(subproc).ekill();
        subproc_stdout
    }};
    
    // cmd syntax into expr
    (@cmd($input:expr, $workdir:expr, $($t:tt)*))=>{
        $crate::cmd_util::exec_command(
            $input, $workdir, format!($($t)*))
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

/// Spawn a thread to delegate from a `Read` to our
/// `stdout`.
pub fn printout<R>(read: R)
where
    R: Read + Send + 'static {
    
    thread::Builder::new()
        .name("cmd_util::printout delegate thread".into())
        .spawn(move || {
            let read = BufReader::new(read);
            for line in read
                .lines()
                .map(|item| match item {
                    Ok(s) => s,
                    Err(e) => format!("{:?}", e),
                })
                .flat_map(|s| s.lines()
                    .map(String::from)
                    .collect::<Vec<_>>())
            {
                println!("| {}", line);
            }
        })
        .ekill();
}

/// Read a line from a process's output.
pub fn preadln(stdout: ChildStdout) -> String {
    BufReader::new(stdout).lines().next()
        .ok_or_else(|| format_err!("subprocess did not \
            print anything"))
        .ekill().ekill()
}

/// Read a sequence of lines from a process's output.
pub fn preadlns(stdout: ChildStdout) -> Vec<String> {
    BufReader::new(stdout).lines()
        .collect::<Result<Vec<_>, _>>()
        .ekill()
}

/// Read the process's output, return whether it printed
/// any non-whitespace character.
pub fn pnonempty(mut stdout: ChildStdout) -> bool {
    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf).ekill();
    let out = String::from_utf8(buf).ekill();
    out.trim().len() > 0
}

/// Join a process, return its exit code.
pub fn pjoin(mut child: Child) -> Result<(), Error> {
    let status = child.wait().map_err(Error::from)?;
    if status.success() {
        Ok(())
    } else {
        Err(format_err!("exit code {:?}", status.code()))
    }
}

pub fn exec_command<I, P, C>(
    input: I, workdir: P, cmd: C)
    -> (Child, ChildStdout)
where 
    I: Read + Send + 'static,
    P: AsRef<Path>, 
    C: AsRef<str> 
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
        .stderr(Stdio::piped())
        .current_dir(&workdir);    
    let sys_cmd_str = format!("{:?}", sys_cmd);
    //printbl!("[DEBUG] ", "executing command:\n{}", sys_cmd_str); 
    let mut subproc = sys_cmd.spawn().ekill();
    let subproc_in = subproc.stdin.take().unwrap();
    let subproc_out = subproc.stdout.take().unwrap();
    
    printout(subproc.stderr.take().unwrap());
    
    // spawn thread to pipe in the stdin content
    thread::Builder::new()
        .name(format!("thread to pipe input into {}",
            sys_cmd_str))
        .spawn(move || {
            let mut pipe_from = BufReader::new(input);
            let mut pipe_into = BufWriter::new(subproc_in);
            
            loop {
                let chunk = pipe_from.fill_buf()
                    .map_err(|e| format!("error reading from stdin_content:\n\
                        {}\
                        to subprocess:\
                        {}", e, sys_cmd_str))
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
                            {}", e, sys_cmd_str);
                        break;
                    },
                };
                pipe_from.consume(released);
            }
        })
        .ekill();
      
    // exit
    (subproc, subproc_out)
}

