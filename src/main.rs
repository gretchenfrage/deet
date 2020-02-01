
extern crate failure;
//extern crate cmd_lib;
extern crate unicode_segmentation;
extern crate rand;
extern crate regex;

/// Helpers for cli to the tool.
#[macro_use] pub mod cli_util;
/// Helpers for running subcommands.
#[macro_use] pub mod cmd_util;
pub mod hex;

use crate::{
    hex::Hex,
    cli_util::{
        parse_var,
        ResultExt,
    },
};
use std::{
    path::PathBuf,
    fs::canonicalize,
};
use rand::prelude::*;

/// Check subcommand.
fn check() {
    let tmp: PathBuf = parse_var("DEET_TMP_DIR").ekill();
    let tmp = canonicalize(&tmp).ekill();
    printbl!("- ", "Executing deet check");
    printbl!("- ", "Using temp directory:\n{:?}", &tmp);
    
    // scratch repo path
    let srp: PathBuf = tmp.join(format!("srp-{}", random::<Hex>()));
    printbl!("- ", "Creating scratch repo in:\n{:?}", srp);

    printbl!("- ", "executing command");
    exec!(&tmp, r#"OBJ=world printenv" "#);
}

fn main() {
    match_args!(match {
        &["check"] => check(),
        _ => kill!("illegal cli args"),
    });
}