
extern crate failure;
//extern crate cmd_lib;
extern crate unicode_segmentation;
extern crate rand;
extern crate regex;
extern crate cargo;

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
    cmd_util::{
        preadln,
    },
};
use std::{
    path::{PathBuf, Path},
    fs::{
        canonicalize,
        create_dir as mkdir,
    },
    ffi::OsStr,
};
use rand::prelude::*;

/// Check subcommand.
fn check<P: AsRef<OsStr>>(package: P) {
    printbl!("- ", "Executing DEET check");

    // recreate in a new git repo
    
    let pckg = PathBuf::from(package.as_ref());
    let pckg = canonicalize(&pckg).ekill();
    printbl!("- ", "For package at:\n{:?}", pckg);
    
    let pckg_repo = exec!(
        [&pckg, "git rev-parse --show-toplevel"] 
        | (preadln)
    );
    printbl!("- ", "Using the repo at:\n{:?}", pckg_repo);
    
    let pckg_branch = exec!(
        [&pckg, "git rev-parse --abbrev-ref HEAD"]
        | (preadln)
    );
    printbl!("- ", "Which is in branch {:?}", pckg_branch);

    let tmp: PathBuf = parse_var("DEET_TMP_DIR").ekill();
    let tmp = canonicalize(&tmp).ekill();
    printbl!("- ", "Using temp directory:\n{:?}", &tmp);
    
    let srp: PathBuf = tmp.join(format!("srp-{}", random::<Hex>()));
    printbl!("- ", "Creating scratch repo in:\n{:?}", srp);
    
    mkdir(&srp).ekill();
    exec!([&srp, "git init"]);
    exec!([&srp, "git remote add local {:?}", pckg_repo]);
    exec!([&srp, "git fetch local"]);
    exec!([&srp, "git checkout local/{}", pckg_branch]);
    exec!([&pckg_repo, "git diff"] | [&srp, "git apply"]);
    
    use cargo::{
        util::config::Config,
        core::Workspace,
    };
    use failure::{Error, format_err};
    
    fn path_rebase<P0, P1, P2>(full: P0, old_base: P1, new_base: P2) -> Result<PathBuf, Error> 
    where
        P0: AsRef<Path>,
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        full.as_ref().strip_prefix(old_base.as_ref())
            .map_err(|e| format_err!(
                "error rebasing paths\n \
                path={:?}\nfrom={:?}\nto={:?}\n\
                error:\n{:#?}", 
                full.as_ref(), 
                old_base.as_ref(), 
                new_base.as_ref(), 
                e))
            .map(|suffix| new_base.as_ref().join(suffix))
    }
    
    let cargo_cfg = Config::default().ekill();
    let manifest = path_rebase(&pckg, &pckg_repo, &srp)
        .ekill()
        .join("Cargo.toml");
    let workspace = Workspace::new(&manifest, &cargo_cfg)
        .ekill();
    dbg!(&workspace);
    
    
}

fn main() {
    match_args!(match {
        ["check", package] => check(package),
        args => kill!("illegal cli args: {:?}", args),
    });
}