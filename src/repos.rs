use std::{error::Error, fs::File, io::Write, os::unix::prelude::PermissionsExt, path::Path};
use walkdir::WalkDir;

const HOOK_FILE_VERSION: u8 = 1;

pub fn index(dir: impl AsRef<Path>, port: u16) -> Result<(), Box<dyn Error>> {
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

            if pr_hook_dir.exists() {
                // TODO: Check for hook file version and replace it
                //       if there is a new version.
                return Ok(());
            }

            let mut pr_hook_file = File::create(pr_hook_dir)?;
            #[cfg(unix)]
            {
                let mut perms = pr_hook_file.metadata()?.permissions();
                perms.set_mode(perms.mode() | 0o100);
                pr_hook_file.set_permissions(perms)?;
            };

            writeln!(
                pr_hook_file,
                "#!/bin/bash\n\
                \n\
                # This file has been auto-generated by minicd.\n\
                # minicd::hookfile_version {HOOK_FILE_VERSION}\n\
                \n\
                while read old_commit new_commit ref_name; do\n\
                    curl -X POST http://127.0.0.1:{port}/api/postreceive \\\n\
                    \t-d \"{} $new_commit $ref_name\"\n\
                done
                ",
                path.to_string_lossy()
            )?;
        }
    }

    Ok(())
}
