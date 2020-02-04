
use std::path::{Path, PathBuf};
use failure::{Error, format_err};

pub fn path_rebase<P0, P1, P2>(full: P0, old_base: P1, new_base: P2) -> Result<PathBuf, Error> 
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