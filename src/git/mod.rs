#![allow(unused)]

pub mod error;

use error::{Error, Result};
use std::{
    ffi::OsStr,
    process::{Command, Output},
};

#[derive(Clone)]
pub struct Repository {
    remote: String,
    dir: String,
    reference: String,
}

impl Repository {
    pub fn clone<S: Into<String>>(remote: S, dir: S) -> Result<Self> {
        let remote: String = remote.into();
        let dir: String = dir.into();
        let reference: String = "HEAD".into();

        let _ = cmd(["clone", &remote, &dir]);

        Ok(Self {
            remote,
            dir,
            reference,
        })
    }

    pub fn checkout<S: Into<String>>(&mut self, reference: S) -> Result<Output> {
        let reference: String = reference.into();
        let out = cmd(["-C", &self.dir, "checkout", &reference])?;
        self.reference = reference;
        Ok(out)
    }

    pub fn get_ref_name(&self) -> Result<String> {
        cmd(["-C", &self.dir, "describe", "--tags", "--exact-match"])
            .and_then(|v| String::from_utf8(v.stdout).map_err(Error::OutputEncode))
            .or_else(|_| {
                cmd([
                    "-C",
                    &self.dir,
                    "branch",
                    "-a",
                    "--contains",
                    &self.reference,
                ])
                .and_then(|v| String::from_utf8(v.stdout).map_err(Error::OutputEncode))
                .map(|v| {
                    v.split('\n')
                        .nth(1)
                        .unwrap_or(&self.reference)
                        .trim()
                        .to_string()
                })
            })
    }

    pub fn get_remote(&self) -> &str {
        &self.remote
    }

    pub fn get_dir(&self) -> &str {
        &self.dir
    }

    pub fn get_ref(&self) -> &str {
        &self.reference
    }
}

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
