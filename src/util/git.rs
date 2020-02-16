//! Git utilities.

use super::{
    cmd::*,
    cli::*,
};
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;

/// A git commit.
#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub concise: String,
    pub pretty: String,
}

/// List the git commits which effect a file/directory.
pub fn follow<P0, P1>(repo: P0, path: P1) -> Vec<Commit> 
where
    P0: AsRef<Path>,
    P1: AsRef<Path>,
{
    exec!(
        [&repo, r##" git log --format="%h" --follow -- {:?} "##, path.as_ref()]
        | (preadlns))
        .into_iter()
        .map(|hash| {
            let pretty = exec!(
                [&repo, r##" git log --format="* %C(auto)%h %f" -n 1 {} "##, hash]
                | (preadln));
            
            let concise = {
                let msg: String = exec!(
                    [&repo, r##" git log --format="%f" -n 1 {} "##, hash]
                    | (preadln));
                    
                const MAX_LEN: usize = 30;
                
                let mut concise = String::with_capacity(MAX_LEN);
                let mut count = 0;
                for g in msg.graphemes(true).take(MAX_LEN - 1) {
                    concise.push_str(g);
                    count += 1;
                }
                if count == MAX_LEN - 1 {
                    concise.push('…');
                }
                
                format!("{} {:?}", hash, concise)
            };
            
            Commit { hash, pretty, concise }
        })
        .collect()
}