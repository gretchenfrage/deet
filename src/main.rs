
#[macro_use]
extern crate log;

#[macro_use]
pub mod util;
#[macro_use]
pub mod leet;
pub mod maniflect;
/// Changelog parsing.
pub mod changelog;

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
        git,
    },
    maniflect::{ManifestFile, DepSource},
    leet::{
        catch_errors,
        log_indent,
    },
    changelog::read_changelog,
};
use std::{
    path::{PathBuf, Path},
    fs::{
        self,
        canonicalize,
        create_dir as mkdir,
    },
};
use rand::prelude::*;
use semver::{
    Version, 
    VersionReq
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum MoistMeter {
    Dry,
    Wet,
}

/// Check subcommand.
fn run<P: AsRef<str>>(
    package: P,
    version: Option<Version>,
    moist: MoistMeter,
) {
    match moist {
        MoistMeter::Dry => info!("Executing DEET check"),
        MoistMeter::Wet => info!("Publishing crate via DEET"),
    };
    let catch = catch_errors(false);
    
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
    if pckg_branch != "main" {
        match moist {
            MoistMeter::Dry => warn!("Repo is not in main branch"),
            MoistMeter::Wet => kill!("Repo is not in main branch"),
        };
    }
    if exec!(
        [&pckg_repo, "git log origin/{}..HEAD", pckg_branch] | (pnonempty)
    ) {
        match moist {
            MoistMeter::Dry => warn!("Repo has unpushed commits"),
            MoistMeter::Wet => kill!("Repo has unpushed commits"),
        };
    }
    if exec!(
        [&pckg_repo, "git log HEAD..origin/{}", pckg_branch] | (pnonempty)
    ) {
        match moist {
            MoistMeter::Dry => warn!("Repo is behind origin"),
            MoistMeter::Wet => kill!("Repo is behind origin"),
        };
    }
    let tmp: PathBuf = parse_var("DEET_TMP_DIR").ekill();
    let tmp = canonicalize(&tmp).ekill();
    debug!("Using temp directory:\n{:?}", &tmp);
    let srp: PathBuf = tmp.join(format!("srp-{}", random::<Hex>()));
    debug!("Creating scratch repo in:\n{:?}", srp);
    
    mkdir(&srp).ekill();
    exec!([&srp, "git init"]);
    match moist {
        MoistMeter::Dry => {
            // pull from local, and move over local changes
            exec!([&srp, "git remote add local {:?}", pckg_repo]);
            exec!([&srp, "git fetch local"]);
            exec!([&srp, "git -c advice.detachedHead=false checkout local/{}", pckg_branch]);
            
            let mut local_changes = false;
            if exec!([&pckg_repo, "git diff"] | (pnonempty)) {
                exec!([&pckg_repo, "git diff"] | [&srp, "git apply"]);
                local_changes = true;
            }
            for path in exec!(
                [&pckg_repo, "git ls-files --others --exclude-standard"] 
                | (preadlns)) 
            {
                fs::create_dir_all(srp.join(&path).parent().unwrap()).ekill();
                fs::copy(
                    Path::new(&pckg_repo).join(&path), 
                    srp.join(&path)
                ).ekill();
                local_changes = true;
            }
            if local_changes {
                warn!("Uncommitted local changes copied over.");
                if version.is_some() {
                    trace!("Creating commit for copied over local changes");
                    exec!([&srp, "git add ."]);
                    exec!([&srp, r#"git commit -m "(local changes copied over by DEET)""#])
                }
            }
        },
        MoistMeter::Wet => {
            // stopgap
            if exec!(
                [&pckg_repo, "git diff"] | (pnonempty)
            ) {
                kill!("There are uncommitted local changes:\n{}",
                    Lines(exec!([&pckg_repo, "git diff"] | (preadlns))));
            }
            
            if exec!(
                [&pckg_repo, "git ls-files --others --exclude-standard"] | (pnonempty)
            ) {
                kill!("There are uncommitted local new files:\n{}",
                    Lines(exec!([&pckg_repo, "git ls-files --others --exclude-standard"] | (preadlns))));
            }
            
            let origin = exec!([&pckg_repo, "git config --get remote.origin.url"] | (preadln));
            info!("Pulling from {}", origin);
            exec!([&srp, "git remote add origin {:?}", origin]);
            exec!([&srp, "git fetch origin"]);
            exec!([&srp, "git checkout origin/{}", pckg_branch]);
        },
    };
    
    
    // ==== de-localize paths ====
    
    let package_path = path_rebase(&pckg, &pckg_repo, &srp)
        .ekill();
    let manifest_path = package_path.join("Cargo.toml");
    info!("Delocalizing manifest at:\n{:?}", manifest_path);

    let indent = log_indent();
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

        indent.linebreak();
        info!("De-localizing dependency {:?} at:\n{:?}", dep.package(), local_path);
        
        // list relevant commits
        let commits = git::follow(&srp, &local_path);
        
        debug!("Found relevant commits:\n{}", 
            LinesView(&commits, |c| &c.pretty));
        
        let latest_commit = commits.get(0)
            .unwrap_or_else(|| kill!(
                "You silly goose!\nThis repo doesn't have any commits"));
        let tags: Vec<String> = exec!(
            [&srp, "git tag --points-at {}", latest_commit.hash]
            | (preadlns));
        
        info!("Looking at latest commit: {}", latest_commit.concise);
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
    indent.end();
    manifest_file.save().ekill();
    
    // run checks
    info!("Running cargo check");
    exec!([&package_path, "cargo check --color always"]);
    
    info!("Running cargo test");
    exec!([&package_path, "cargo test --color always"]);

    info!("Running cargo doc");
    exec!([&package_path, "cargo doc --no-deps --document-private-items --color always"]);
    
    let changelog_path = package_path.join("CHANGELOG.md");
    info!("Reading changelog at {:?}", changelog_path);
    let changelog = read_changelog(&changelog_path)
        .map_err(|e| kill!("error reading changelog:\n{}", e))
        .unwrap();
    debug!("Changelog: \n\n{}", Lines(&changelog));
    
    let version = match version {
        None => {
            info!("Since no version to release was specified, the check is ending now.");
            return catch.handle(true);
        },
        Some(v) => v,
    };
    
    let version_note = changelog
        .iter()
        .find(|e| e.version == version)
        .cloned();
    let version_note = match version_note {
        Some(n) => n,
        None => {
            kill!("Could not find version {} in changelog", version);
        },
    };

    let package_name = manifest_file.name().ekill();
    info!("Package name = {}", package_name);

    info!("Current version = {}", manifest_file.version().ekill());
    info!("Found version {} in changelog:\n{}", version, version_note);
    
    debug!("Altering version in manifest at:\n{:?}", manifest_path);
    manifest_file.set_version(&version.to_string()).ekill();
    manifest_file.save().ekill();
    
    // make a new commit
    let publish_tag = format!("{}-v{}", package_name, version);
    info!("Creating new commit and tagging {}", publish_tag);
    exec!([&srp, "git add {:?}", manifest_path]);
    exec!([&srp, r#"git commit -m "Publish {}""#, publish_tag]);
    exec!([&srp, "git tag {} HEAD", publish_tag]);

    match moist {
        MoistMeter::Dry => {
            info!("Running cargo publish dry run");
            exec!([package_path, "cargo publish  --color always --locked --dry-run --allow-dirty"]);

            catch.handle(false);
        },
        MoistMeter::Wet => {
            catch.handle(false);
            
            info!("Publishing to crates.io");
            exec!([package_path, "cargo publish --color always --locked"]);
            
            color!(green "[ INFO  ] Successfully published, committing and pushing.";,);
            manifest_file.set_version(&format!("{}-AFTER", version)).ekill();
            exec!([&srp, "git add {:?}", manifest_path]);
            exec!([&srp, r#"git commit -m "After-release {}""#, publish_tag]);
            exec!([&srp, "git checkout -b {}", pckg_branch]);
            exec!([&srp, "git push -u origin {0}:{0}", pckg_branch]);
            exec!([&srp, "git push -u origin {0}:{0}", publish_tag]);
            exec!([&pckg_repo, "git fetch origin"]);
            exec!([&pckg_repo, "git pull origin {}", pckg_branch]);
            exec!([&pckg_repo, "git pull origin {}", publish_tag]);
        }
    };
    
    color!("\n";green "[ EXIT  ] Process successful.";"\n";,);
}

fn parse_release_tag(tag: &str, package: &str) -> Option<Version> {
    tag.strip_prefix(package)
        .and_then(|s| s.strip_prefix("-v"))
        .and_then(|s| Version::parse(s).ok())
}

fn main() {
    leet::init_from_env();
        
    match_args!(match {
        [] | ["--help"] => println!("{}", include_str!("../README.txt").trim()),
        ["check", package] => run(package, None, MoistMeter::Dry),
        ["check", package, version] => {
            let version = version.parse::<Version>().ekill();
            run(package, Some(version), MoistMeter::Dry);
        },
        ["publish", package, version] => {
            let version = version.parse::<Version>().ekill();
            run(package, Some(version), MoistMeter::Wet);
        },
        args => kill!("illegal cli args: {:?}", args),
    });
}