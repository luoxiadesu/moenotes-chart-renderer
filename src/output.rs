//! Transactional publication. Immutable generation + atomic pointer is the
//! authoritative page set; legacy sibling PNGs are rollback-protected aliases.
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
fn write_sync(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new().create_new(true).write(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn basename(name: &str) -> bool {
    let p = Path::new(name);
    !name.is_empty()
        && !name.contains(['/', '\\', ':'])
        && p.file_name().is_some_and(|n| n == p.as_os_str())
        && !name.starts_with('.')
}
fn replace_pointer(stage: &Path, current: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        fs::rename(stage, current)?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let a: Vec<_> = stage.as_os_str().encode_wide().chain([0]).collect();
        let b: Vec<_> = current.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: valid NUL-terminated paths; replace existing and write-through flags.
        ensure!(
            unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 1 | 8) } != 0,
            "Atomic pointer replacement failed: {}",
            std::io::Error::last_os_error()
        );
    }
    #[cfg(not(any(unix, windows)))]
    {
        anyhow::bail!("Transactional output unsupported on this OS");
    }
    Ok(())
}
/// OutputGroup is an explicit set. `current.json` is the atomic commit point;
/// consumers requiring crash consistency read that pointer, not loose aliases.
pub fn publish(
    output: &Path,
    files: Vec<(String, Vec<u8>)>,
    protected: &[PathBuf],
) -> Result<PathBuf> {
    publish_impl(output, files, protected, None)
}
fn publish_impl(
    output: &Path,
    files: Vec<(String, Vec<u8>)>,
    protected: &[PathBuf],
    fail_after: Option<usize>,
) -> Result<PathBuf> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let stem = output
        .file_stem()
        .context("Output stem")?
        .to_str()
        .context("Output filename must be UTF-8")?;
    let group = parent.join(format!("{stem}.render-set"));
    ensure!(!group.is_symlink(), "Refusing symlink output group");
    fs::create_dir_all(&group)?;
    ensure!(
        !group.join("write.lock").is_symlink() && !group.join("current.json").is_symlink(),
        "Refusing symlink group metadata"
    );
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(group.join("write.lock"))?;
    lock.try_lock_exclusive()
        .context("Output group is already being written")?;
    for (name, _) in &files {
        ensure!(basename(name), "Unsafe output filename");
        let p = parent.join(name);
        ensure!(!p.is_symlink(), "Refusing symlink output {}", p.display());
        if p.exists() {
            let canon = p.canonicalize()?;
            for input in protected {
                if input.exists() {
                    ensure!(
                        canon != input.canonicalize()?,
                        "Output would overwrite input {}",
                        input.display()
                    );
                }
            }
        }
    }
    let current = group.join("current.json");
    let old: Option<Value> = if current.exists() {
        Some(serde_json::from_slice(&fs::read(&current)?)?)
    } else {
        None
    };
    let oldnames: Vec<String> = old
        .as_ref()
        .and_then(|v| v["files"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let mut unique = std::collections::BTreeSet::new();
    for (n, _) in &files {
        ensure!(unique.insert(n), "Duplicate output filename {n}");
    }
    ensure!(
        oldnames.iter().all(|n| basename(n)),
        "Corrupt previous output manifest"
    );
    for n in &oldnames {
        let dest = parent.join(n);
        ensure!(!dest.is_symlink(), "Refusing symlink previous output");
        if dest.exists() {
            ensure!(dest.is_file(), "Output is not a regular file");
            for input in protected {
                if input.exists() {
                    ensure!(
                        dest.canonicalize()? != input.canonicalize()?,
                        "Previous page would overwrite protected input"
                    );
                }
            }
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let generation = format!("generation-{stamp}-{}", std::process::id());
    let dir = group.join(&generation);
    fs::create_dir(&dir)?;
    // An uncommitted generation may survive a crash, but current.json still points
    // to the previous complete set. Never destroy that set during publication.
    let prepare = (|| -> Result<()> {
        for (name, data) in &files {
            write_sync(&dir.join(name), data)?;
        }
        Ok(())
    })();
    if let Err(e) = prepare {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    let mut names = oldnames;
    for (n, _) in &files {
        if !names.contains(n) {
            names.push(n.clone());
        }
    }
    ensure!(
        names.iter().all(|n| basename(n)),
        "Corrupt previous output manifest"
    );
    let backup = dir.join(".backup");
    fs::create_dir(&backup)?;
    let mut backed = vec![];
    let mut installed = vec![];
    let commit = (|| -> Result<()> {
        for n in &names {
            let dest = parent.join(n);
            if dest.exists() {
                ensure!(
                    !dest.is_symlink() && dest.is_file(),
                    "Unsafe previous output"
                );
                fs::rename(&dest, backup.join(n))?;
                backed.push(n.clone());
            }
        }
        for (n, _) in &files {
            installed.push(n.clone());
            fs::copy(dir.join(n), parent.join(n))?;
            ensure!(
                fail_after != Some(installed.len()),
                "Injected publication failure"
            );
        }
        let manifest = json!({"schema_version":1,"generation":generation,"files":files.iter().map(|(n,b)|json!({"name":n,"sha256":crate::sha256(b),"bytes":b.len()})).collect::<Vec<_>>()});
        let pointer = group.join(format!(".pointer-{stamp}"));
        write_sync(&pointer, &serde_json::to_vec_pretty(&manifest)?)?;
        #[cfg(unix)]
        {
            std::fs::File::open(&dir)?.sync_all()?;
        }
        replace_pointer(&pointer, &current)?;
        #[cfg(unix)]
        {
            let _ = std::fs::File::open(&group).and_then(|f| f.sync_all());
        }
        Ok(())
    })();
    if let Err(e) = commit {
        for n in installed {
            let _ = fs::remove_file(parent.join(n));
        }
        for n in backed {
            let _ = fs::rename(backup.join(&n), parent.join(n));
        }
        return Err(e);
    }
    let _ = fs::remove_dir_all(&backup);
    // Keep the previous immutable generation for rollback/audit; remove no
    // unrelated files. Loose aliases from the old page count are already gone.
    FileExt::unlock(&lock)?;
    Ok(current)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaces_set_removes_stale_and_protects_input() -> Result<()> {
        let t = std::env::temp_dir().join(format!(
            "moenotes-output-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&t)?;
        let output = t.join("x.png");
        publish(
            &output,
            vec![("x-001.png".into(), vec![1]), ("x-002.png".into(), vec![2])],
            &[],
        )?;
        publish(&output, vec![("x.png".into(), vec![3])], &[])?;
        assert!(!t.join("x-001.png").exists());
        assert!(!t.join("x-002.png").exists());
        let before = fs::read(t.join("x.render-set/current.json"))?;
        assert!(
            publish(
                &output,
                vec![("x.png".into(), vec![4])],
                std::slice::from_ref(&output)
            )
            .is_err()
        );
        assert_eq!(fs::read(&output)?, [3]);
        assert_eq!(fs::read(t.join("x.render-set/current.json"))?, before);
        fs::remove_dir_all(t)?;
        Ok(())
    }
    #[test]
    fn mid_commit_failure_restores_previous_aliases_and_pointer() -> Result<()> {
        let t = std::env::temp_dir().join(format!(
            "moenotes-rollback-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&t)?;
        let path = t.join("x.png");
        publish(
            &path,
            vec![("x.png".into(), vec![1]), ("x.render.json".into(), vec![2])],
            &[],
        )?;
        let pointer = fs::read(t.join("x.render-set/current.json"))?;
        assert!(
            publish_impl(
                &path,
                vec![("x.png".into(), vec![8]), ("x.render.json".into(), vec![9])],
                &[],
                Some(1)
            )
            .is_err()
        );
        assert_eq!(fs::read(t.join("x.png"))?, [1]);
        assert_eq!(fs::read(t.join("x.render.json"))?, [2]);
        assert_eq!(fs::read(t.join("x.render-set/current.json"))?, pointer);
        fs::remove_dir_all(t)?;
        Ok(())
    }
}
