#![feature(str_strip)]

extern crate failure;
extern crate unicode_segmentation;
extern crate rand;
extern crate regex;
extern crate toml_edit;
extern crate semver;

/// Helpers for cli to the tool.
#[macro_use] pub mod cli_util;
/// Helpers for running subcommands.
#[macro_use] pub mod cmd_util;
pub mod hex;
pub mod manifest;
pub mod path_util;

use crate::{
    hex::Hex,
    cli_util::{
        parse_var,
        ResultExt,
        Lines, GetLines,
    },
    cmd_util::{
        preadln, preadlns, pnonempty
    },
    path_util::path_rebase,
    manifest::{ManifestFile, Dep, DepSource, DepKey},
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
use semver::{Version, VersionReq};


/// Check subcommand.
fn check<P: AsRef<OsStr>>(package: P) {
    printbl!("- ", "Executing DEET check");

    // ==== recreate in a new git repo ====
    
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
    
    if exec!([&pckg_repo, "git diff"] | (pnonempty)) {
        exec!([&pckg_repo, "git diff"] | [&srp, "git apply"]);
    }
    
    // ==== de-localize paths ====
    
    let package_path = path_rebase(&pckg, &pckg_repo, &srp)
        .ekill();
    let manifest_path = package_path.join("Cargo.toml");
    
    printbl!("- ", "Delocalizing manifest at:\n{:?}", manifest_path);
    
    let mut manifest_file = ManifestFile::new(&manifest_path).ekill();
    for mut dep in manifest_file.deps().ekill() {
        // get and canonicalize the local path
        let local_path = match dep.source().local_path().map(Path::new) {
            Some(path) => {
                if path.is_relative() {
                    canonicalize(package_path.join(&path)).ekill()
                } else {
                    canonicalize(path).ekill()
                }
            },
            None => continue,
        };
        
        printbl!("-- ", "De-localizing dependency:\n{:#?}\nAt:\n{:?}", dep, local_path);
        
        // list relevant commits
        #[derive(Debug, Clone)]
        struct Commit {
            hash: String,
            pretty: String,
        }
        
        let commits: Vec<Commit> = exec!(
            [&srp, r##" git log --format="%h" --follow -- {:?} "##, local_path]
            | (preadlns))
            .into_iter()
            .map(|hash| {
                let pretty = exec!(
                    [&srp, r##" git log --format="* %C(auto)%h %f" -n 1 {} "##, hash]
                    | (preadln));
                Commit { hash, pretty }
            })
            .collect();
        
        printbl!("-- ", "Found relevant commits:\n{}", 
            GetLines(&commits, |c| &c.pretty));
        
        let latest_commit = commits.get(0)
            .unwrap_or_else(|| kill!(
                "You silly goose!\nThis repo doesn't have any commits"));
        let tags: Vec<String> = exec!(
            [&srp, "git tag --points-at {}", latest_commit.hash]
            | (preadlns));
        
        printbl!("-- ", "Looking at latest commit: {}", latest_commit.hash);
        printbl!("-- ", "Found tags on commit:\n{}", Lines(&tags));
        
        let versions: Vec<Version> = tags.iter()
            .filter_map(|tag| 
                parse_release_tag(tag, dep.package()))
            .collect();
        
        // select the version
        let version = match versions.as_slice() {
            &[] => /* TODO */ { eprintln!("No versions found on commit"); continue },
            &[ref v] => v.clone(),
            _ => { eprintln!("Several versions found on commit:\n{}", Lines(&versions)); continue },
        };
        
        printbl!("-- ", "Found version {}", version);
        
        let version_req = format!(
            "{}", VersionReq::parse(&format!(
                "^{}", version)).ekill());
                
        printbl!("-- ", "Replacing local dep with version req {}", version_req);
        
        dep.set_source(DepSource::Crates {
            version: version_req,
        });
    }
    
    manifest_file.save().ekill();
    
    println!("hell yes!");
}

fn parse_release_tag(tag: &str, package: &str) -> Option<Version> {
    tag.strip_prefix(package)
        .and_then(|s| s.strip_prefix("-v"))
        .and_then(|s| Version::parse(s).ok())
}

fn main() {
    match_args!(match {
        ["check", package] => check(package),
        args => kill!("illegal cli args: {:?}", args),
    });
}