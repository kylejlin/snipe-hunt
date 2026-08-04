use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Eq, PartialEq)]
pub struct Publication {
    pub previous_version: String,
    pub version: String,
}

/// Rejects publication from a dirty Git worktree unless the caller explicitly opts out.
pub fn require_clean_worktree(
    repository_directory: &Path,
    allow_when_dirty: bool,
) -> io::Result<()> {
    if allow_when_dirty {
        return Ok(());
    }

    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=normal"])
        .current_dir(repository_directory)
        .output()
        .map_err(|error| io::Error::other(format!("failed to inspect Git worktree: {error}")))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "failed to inspect Git worktree{}",
            if detail.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", detail.trim())
            }
        )));
    }
    if !output.stdout.is_empty() {
        return Err(io::Error::other(
            "refusing to publish from a dirty Git worktree; commit or stash the changes, or pass --allow-when-dirty",
        ));
    }
    Ok(())
}

/// Publishes an embedded browser model and advances the version displayed by the web app.
///
/// All input is validated and every replacement file is written before the first tracked file
/// changes. If a replacement fails, files already replaced during this call are restored.
pub fn publish_model(
    destination: &Path,
    model_bytes: &[u8],
    web_directory: &Path,
) -> io::Result<Publication> {
    let package_path = web_directory.join("package.json");
    let lock_path = web_directory.join("package-lock.json");
    let package_bytes = fs::read(&package_path)?;
    let lock_bytes = fs::read(&lock_path)?;
    let mut package = parse_json(&package_path, &package_bytes)?;
    let mut lock = parse_json(&lock_path, &lock_bytes)?;

    let previous_version = string_at(&package_path, &package, &["version"])?;
    let lock_version = string_at(&lock_path, &lock, &["version"])?;
    let lock_package_version = string_at(&lock_path, &lock, &["packages", "", "version"])?;
    if lock_version != previous_version || lock_package_version != previous_version {
        return Err(invalid(format!(
            "web package versions disagree: package.json={previous_version}, package-lock.json={lock_version}, package-lock root={lock_package_version}"
        )));
    }

    let version = next_minor_version(&previous_version)?;
    set_string_at(&package_path, &mut package, &["version"], &version)?;
    set_string_at(&lock_path, &mut lock, &["version"], &version)?;
    set_string_at(
        &lock_path,
        &mut lock,
        &["packages", "", "version"],
        &version,
    )?;

    let package_bytes = pretty_json(&package_path, &package)?;
    let lock_bytes = pretty_json(&lock_path, &lock)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    } else {
        return Err(invalid("invalid model publication path"));
    }

    // package.json is the GUI's version source, so replace it last. Its new value acts as the
    // commit marker for the model and package-lock replacements that precede it.
    replace_all_or_restore(&[
        Replacement::prepare(destination, model_bytes)?,
        Replacement::prepare(&lock_path, &lock_bytes)?,
        Replacement::prepare(&package_path, &package_bytes)?,
    ])?;

    Ok(Publication {
        previous_version,
        version,
    })
}

fn parse_json(path: &Path, bytes: &[u8]) -> io::Result<Value> {
    serde_json::from_slice(bytes).map_err(|error| invalid(format!("{}: {error}", path.display())))
}

fn pretty_json(path: &Path, value: &Value) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| invalid(format!("{}: {error}", path.display())))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn string_at(path: &Path, value: &Value, keys: &[&str]) -> io::Result<String> {
    let mut current = value;
    for key in keys {
        current = current
            .get(key)
            .ok_or_else(|| invalid(format!("{} is missing {}", path.display(), keys.join("."))))?;
    }
    current.as_str().map(str::to_owned).ok_or_else(|| {
        invalid(format!(
            "{} has a non-string {}",
            path.display(),
            keys.join(".")
        ))
    })
}

fn set_string_at(path: &Path, value: &mut Value, keys: &[&str], new: &str) -> io::Result<()> {
    let mut current = value;
    for key in &keys[..keys.len().saturating_sub(1)] {
        current = current
            .get_mut(key)
            .ok_or_else(|| invalid(format!("{} is missing {}", path.display(), keys.join("."))))?;
    }
    let Some(last) = keys.last() else {
        return Err(invalid("empty JSON path"));
    };
    let slot = current
        .get_mut(last)
        .ok_or_else(|| invalid(format!("{} is missing {}", path.display(), keys.join("."))))?;
    if !slot.is_string() {
        return Err(invalid(format!(
            "{} has a non-string {}",
            path.display(),
            keys.join(".")
        )));
    }
    *slot = Value::String(new.to_owned());
    Ok(())
}

fn next_minor_version(version: &str) -> io::Result<String> {
    let components = version.split('.').collect::<Vec<_>>();
    let [major, minor, _patch] = components.as_slice() else {
        return Err(invalid(format!(
            "web version {version:?} is not major.minor.patch"
        )));
    };
    let major = major
        .parse::<u64>()
        .map_err(|_| invalid(format!("invalid web version {version:?}")))?;
    let minor = minor
        .parse::<u64>()
        .map_err(|_| invalid(format!("invalid web version {version:?}")))?
        .checked_add(1)
        .ok_or_else(|| invalid("web minor version overflow"))?;
    _patch
        .parse::<u64>()
        .map_err(|_| invalid(format!("invalid web version {version:?}")))?;
    Ok(format!("{major}.{minor}.0"))
}

struct Replacement {
    path: PathBuf,
    temporary: PathBuf,
    original: Option<Vec<u8>>,
}

impl Replacement {
    fn prepare(path: &Path, bytes: &[u8]) -> io::Result<Self> {
        let original = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let temporary = temporary_path(path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(Self {
            path: path.to_owned(),
            temporary,
            original,
        })
    }

    fn restore(&self) -> io::Result<()> {
        match &self.original {
            Some(bytes) => {
                let rollback = temporary_path(&self.path);
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&rollback)?;
                if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&rollback);
                    return Err(error);
                }
                fs::rename(rollback, &self.path)
            }
            None => match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
        }
    }
}

impl Drop for Replacement {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temporary);
    }
}

fn replace_all_or_restore(replacements: &[Replacement]) -> io::Result<()> {
    for (index, replacement) in replacements.iter().enumerate() {
        if let Err(error) = fs::rename(&replacement.temporary, &replacement.path) {
            let mut rollback_errors = Vec::new();
            for committed in replacements[..index].iter().rev() {
                if let Err(rollback_error) = committed.restore() {
                    rollback_errors.push(format!("{}: {rollback_error}", committed.path.display()));
                }
            }
            if rollback_errors.is_empty() {
                return Err(error);
            }
            return Err(io::Error::other(format!(
                "publication failed: {error}; rollback also failed for {}",
                rollback_errors.join(", ")
            )));
        }
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let id = NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("publication");
    path.with_file_name(format!(".{name}.publish-{}-{id}.tmp", std::process::id()))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn publishes_model_and_advances_all_web_versions() {
        let root = test_directory("publish");
        let web = root.join("web");
        let destination = root.join("agent/model.bin");
        fs::create_dir_all(&web).unwrap();
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(
            web.join("package.json"),
            "{\n  \"name\": \"web\",\n  \"version\": \"2.7.9\"\n}\n",
        )
        .unwrap();
        fs::write(
            web.join("package-lock.json"),
            "{\n  \"name\": \"web\",\n  \"version\": \"2.7.9\",\n  \"packages\": {\n    \"\": {\n      \"name\": \"web\",\n      \"version\": \"2.7.9\"\n    }\n  }\n}\n",
        )
        .unwrap();
        fs::write(&destination, b"old weights").unwrap();

        let publication = publish_model(&destination, b"new weights", &web).unwrap();

        assert_eq!(
            publication,
            Publication {
                previous_version: "2.7.9".to_owned(),
                version: "2.8.0".to_owned(),
            }
        );
        assert_eq!(fs::read(destination).unwrap(), b"new weights");
        let package: Value =
            serde_json::from_slice(&fs::read(web.join("package.json")).unwrap()).unwrap();
        let lock: Value =
            serde_json::from_slice(&fs::read(web.join("package-lock.json")).unwrap()).unwrap();
        assert_eq!(package["version"], "2.8.0");
        assert_eq!(lock["version"], "2.8.0");
        assert_eq!(lock["packages"][""]["version"], "2.8.0");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_disagreeing_versions_before_changing_the_model() {
        let root = test_directory("mismatch");
        let web = root.join("web");
        let destination = root.join("model.bin");
        fs::create_dir_all(&web).unwrap();
        fs::write(web.join("package.json"), "{\"version\":\"0.83.0\"}").unwrap();
        fs::write(
            web.join("package-lock.json"),
            "{\"version\":\"0.82.0\",\"packages\":{\"\":{\"version\":\"0.82.0\"}}}",
        )
        .unwrap();
        fs::write(&destination, b"old weights").unwrap();

        let error = publish_model(&destination, b"new weights", &web).unwrap_err();

        assert!(error.to_string().contains("versions disagree"));
        assert_eq!(fs::read(destination).unwrap(), b"old weights");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserves_the_repository_manifest_format() {
        let root = test_directory("real-manifests");
        let web = root.join("web");
        let repository_web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
        fs::create_dir_all(&web).unwrap();
        let original_package = fs::read(repository_web.join("package.json")).unwrap();
        let original_lock = fs::read(repository_web.join("package-lock.json")).unwrap();
        fs::write(web.join("package.json"), &original_package).unwrap();
        fs::write(web.join("package-lock.json"), &original_lock).unwrap();

        let publication = publish_model(&root.join("model.bin"), b"weights", &web).unwrap();

        let old = format!("\"{}\"", publication.previous_version);
        let new = format!("\"{}\"", publication.version);
        let restored_package = fs::read_to_string(web.join("package.json"))
            .unwrap()
            .replace(&new, &old);
        let restored_lock = fs::read_to_string(web.join("package-lock.json"))
            .unwrap()
            .replace(&new, &old);
        assert_eq!(restored_package.as_bytes(), original_package);
        assert_eq!(restored_lock.as_bytes(), original_lock);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dirty_worktrees_are_rejected_unless_explicitly_allowed() {
        let root = test_directory("dirty-worktree");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "--quiet"]);
        fs::write(root.join("tracked.txt"), "clean\n").unwrap();
        git(&root, &["add", "tracked.txt"]);
        git(
            &root,
            &[
                "-c",
                "user.name=Agent publisher test",
                "-c",
                "user.email=agent-publisher@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Initial",
            ],
        );
        require_clean_worktree(&root, false).unwrap();

        fs::write(root.join("tracked.txt"), "dirty\n").unwrap();
        let error = require_clean_worktree(&root, false).unwrap_err();
        assert!(error.to_string().contains("dirty Git worktree"));
        require_clean_worktree(&root, true).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn override_skips_git_inspection_entirely() {
        let missing = test_directory("missing");
        require_clean_worktree(&missing, true).unwrap();
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .status()
            .unwrap();
        assert!(status.success(), "git {arguments:?} failed");
    }

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "agent-publisher-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
