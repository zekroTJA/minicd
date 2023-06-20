pub mod error;

use error::{Error, Result};
use std::{
    ffi::OsStr,
    process::{Command, Output},
};

pub fn cmd<I, S>(cmds: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = Command::new("git")
        .args(cmds)
        .output()
        .map_err(Error::CommandExecution)?;
    if !out.status.success() {
        return Err(out.into());
    }

    Ok(out)
}

pub fn clone(remote: &str, dir: &str) -> Result<Output> {
    cmd(["clone", remote, dir])
}

pub fn checkout(rf: &str, dir: &str) -> Result<Output> {
    cmd(["-C", dir, "checkout", rf])
}
