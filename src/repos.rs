use std::{
    error::Error,
    fs::File,
    io::{Read, Write},
    os::unix::prelude::PermissionsExt,
    path::Path,
};
use walkdir::WalkDir;

const HOOK_MARKER: &str = "# minicd::apicall\n";

pub fn index_repos(dir: impl AsRef<Path>, port: u16) -> Result<(), Box<dyn Error>> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_name() == "HEAD" {
            let Some(path) = entry.path().parent() else {
                continue;
            };

            let hooks_dir = path.join("hooks");
            if !hooks_dir.exists() {
                continue;
            }

            let pr_hook_dir = hooks_dir.join("post-receive");

            let mut pr_hook_file = if pr_hook_dir.exists() {
                let mut f = File::options().read(true).write(true).open(pr_hook_dir)?;
                let mut content = String::new();
                f.read_to_string(&mut content)?;
                if content.contains(HOOK_MARKER) {
                    // TODO: check if the set API call endpoint matches
                    //       matches the configured endpoint address.
                    continue;
                }
                f
            } else {
                let mut f = File::create(pr_hook_dir)?;
                write!(f, "#!/bin/sh\n\n")?;
                #[cfg(unix)]
                {
                    let mut perms = f.metadata()?.permissions();
                    perms.set_mode(perms.mode() | 0o100);
                    f.set_permissions(perms)?;
                };
                f
            };

            writeln!(
                pr_hook_file,
                "{HOOK_MARKER}curl -X POST http://127.0.0.1:{port}/api/postreceive \\\n\
                \t-d \"{} $(git rev-parse HEAD)\"",
                path.to_string_lossy()
            )?;
        }
    }

    Ok(())
}
