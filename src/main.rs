
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
    fs::{
        canonicalize,
        create_dir as mkdir,
    },
    ffi::OsStr,
};
use rand::prelude::*;

/// Check subcommand.
fn check<P: AsRef<OsStr>>(package: P) {
    printbl!("- ", "Executing deet check");

    let pckg = PathBuf::from(package.as_ref());
    let pckg = canonicalize(&pckg).ekill();
    printbl!("- ", "For package at {:?}", pckg);

    let tmp: PathBuf = parse_var("DEET_TMP_DIR").ekill();
    let tmp = canonicalize(&tmp).ekill();
    printbl!("- ", "Using temp directory:\n{:?}", &tmp);
    
    let srp: PathBuf = tmp.join(format!("srp-{}", random::<Hex>()));
    printbl!("- ", "Creating scratch repo in:\n{:?}", srp);
    
    mkdir(&srp).ekill();
    exec!(&srp, r#" git init "#);
    exec!(&srp, r#" git remote add local {:?} "#, pckg);
    exec!(&srp, r#" git fetch local "#);
    exec!(&srp, r#" git checkout local/master "#);
    exec!(&srp, r#" ls "#);
}

fn main() {
    match_args!(match {
        ["check", package] => check(package),
        args => kill!("illegal cli args: {:?}", args),
    });
}