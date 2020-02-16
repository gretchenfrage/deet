#![feature(str_strip)]
#![feature(trace_macros)]
#![feature(backtrace)]

extern crate failure;
extern crate unicode_segmentation;
extern crate rand;
extern crate regex;
extern crate toml_edit;
extern crate semver;
#[macro_use]
extern crate log;
extern crate lazy_static;

/*
/// Helpers for cli to the tool.
#[macro_use] pub mod cli_util;
/// Helpers for running subcommands.
#[macro_use] pub mod cmd_util;
pub mod hex;
pub mod manifest;
pub mod path_util;
/// Command line output handling.
pub mod output;
*/
#[macro_use]
pub mod util;
pub mod leet;
pub mod maniflect;

use crate::{
    util::{
        hex::Hex,
        cli::{
            parse_var,
            ResultExt,
        },
        display::{
            Lines, 
            LinesView,
        },
        cmd::{
            preadln, 
            preadlns,
            pnonempty,
        },
        path::path_rebase,
    },
    maniflect::{ManifestFile, DepSource},
    leet::{
        catch_errors,
    }
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
use semver::{
    Version, 
    VersionReq
};
use unicode_segmentation::UnicodeSegmentation;

/// Check subcommand.
fn check<P: AsRef<OsStr>>(package: P) {
    info!("Executing DEET check");

    // ==== recreate in a new git repo ====
    
    let pckg = PathBuf::from(package.as_ref());
    let pckg = canonicalize(&pckg).ekill();
    debug!("For package at:\n{:?}", pckg);
    
    let pckg_repo = exec!(
        [&pckg, "git rev-parse --show-toplevel"] 
        | (preadln)
    );
    debug!("Using the repo at:\n{:?}", pckg_repo);
    
    let pckg_branch = exec!(
        [&pckg, "git rev-parse --abbrev-ref HEAD"]
        | (preadln)
    );
    debug!("Which is in branch {:?}", pckg_branch);

    let tmp: PathBuf = parse_var("DEET_TMP_DIR").ekill();
    let tmp = canonicalize(&tmp).ekill();
    debug!("Using temp directory:\n{:?}", &tmp);
    
    let srp: PathBuf = tmp.join(format!("srp-{}", random::<Hex>()));
    debug!("Creating scratch repo in:\n{:?}", srp);
    
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
    
    info!("Delocalizing manifest at:\n{:?}", manifest_path);
    
    let catch = catch_errors(true);
    
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
        
        info!("De-localizing dependency {:?} at:\n{:?}", dep.package(), local_path);
        
        // list relevant commits
        #[derive(Debug, Clone)]
        struct Commit {
            hash: String,
            semipretty: String,
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
                    
                let semipretty = {
                    let msg: String = exec!(
                        [&srp, r##" git log --format="%f" -n 1 {} "##, hash]
                        | (preadln));
                    let max_len = 30;
                    let mut concise = String::with_capacity(max_len);
                    let mut count = 0;
                    for g in msg.graphemes(true).take(max_len - 1) {
                        concise.push_str(g);
                        count += 1;
                    }
                    if count == max_len - 1 {
                        concise.push('…');
                    }
                    format!("{} {:?}", hash, concise)
                };
                    
                Commit { hash, pretty, semipretty }
            })
            .collect();
        
        debug!("Found relevant commits:\n{}", 
            LinesView(&commits, |c| &c.pretty));
        
        let latest_commit = commits.get(0)
            .unwrap_or_else(|| kill!(
                "You silly goose!\nThis repo doesn't have any commits"));
        let tags: Vec<String> = exec!(
            [&srp, "git tag --points-at {}", latest_commit.hash]
            | (preadlns));
        
        info!("Looking at latest commit: {}", latest_commit.semipretty);
        debug!("Found tags on commit:\n{}", Lines(&tags));
        
        let versions: Vec<Version> = tags.iter()
            .filter_map(|tag| 
                parse_release_tag(tag, dep.package()))
            .collect();
        
        // select the version
        let version = match versions.as_slice() {
            &[] => { 
                error!("No versions found on commit"); 
                continue;
            },
            &[ref v] => v.clone(),
            _ => { 
                error!("Several versions found on commit:\n{}", Lines(&versions)); 
                continue;
            },
        };
        
        info!("Found version {}", version);
        
        let version_req = format!(
            "{}", VersionReq::parse(&format!(
                "^{}", version)).ekill());
                
        debug!("Replacing local dep with version req {}", version_req);
        
        dep.set_source(DepSource::Crates {
            version: version_req,
        });
    }
    
    catch.handle(true);
    
    manifest_file.save().ekill();
    
    info!("hell yes!");
}

fn parse_release_tag(tag: &str, package: &str) -> Option<Version> {
    tag.strip_prefix(package)
        .and_then(|s| s.strip_prefix("-v"))
        .and_then(|s| Version::parse(s).ok())
}

fn main() {
    leet::init_from_env();
        
    match_args!(match {
        ["check", package] => check(package),
        args => kill!("illegal cli args: {:?}", args),
    });
}