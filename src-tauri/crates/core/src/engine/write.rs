//! Penulisan file yang aman (atomik + permission).
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Konteks error tulis. `Permission denied` diberi jalan keluar siap-tempel:
/// gejala paling umum adalah menulis ke home milik user lain (mis. agent
/// dengan home sendiri di ADE/VPS) dari user yang berbeda.
fn write_context(action: &str, path: &Path) -> impl Fn(std::io::Error) -> anyhow::Error {
    let path = path.to_path_buf();
    let action = action.to_string();
    move |error: std::io::Error| {
        let mut message = format!("{} {}: {}", action, path.display(), error);
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            message.push_str(
                "; this directory likely belongs to another user. Run hakonfig-cli \
                 as the owner of the home, e.g. `sudo -u <user> -H hakonfig-cli ...`, \
                 or pass --home and make sure the directory is writable",
            );
        }
        anyhow::Error::msg(message)
    }
}

/// Tulis ke file sementara di direktori yang sama, lalu `rename`.
/// Ini mencegah file konfigurasi berakhir setengah tertulis bila proses mati.
pub fn atomic_write(path: &Path, contents: &str, sensitive: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| write_context("failed to create directory in", parent)(error))?;
    }

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    let tmp = path.with_file_name(format!(".{}.hakonfig-tmp", file_name));

    {
        let mut file =
            std::fs::File::create(&tmp).map_err(write_context("failed to write", &tmp))?;
        file.write_all(contents.as_bytes())
            .map_err(write_context("failed to write", &tmp))?;
        file.sync_all().ok();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if sensitive { 0o600 } else { 0o644 };
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
        }
    }

    std::fs::rename(&tmp, path).map_err(write_context("failed to replace", path))?;
    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::engine::testutil::temp_dir;

    #[test]
    fn write_creates_parent_directories() {
        let dir = temp_dir("write-basic");
        let path = dir.join("a/b/c.json");
        atomic_write(&path, "{\"x\":1}", false).unwrap();
        assert_eq!(read_to_string(&path).unwrap(), "{\"x\":1}");
    }

    #[test]
    fn overwriting_old_file_leaves_no_temp_file() {
        let dir = temp_dir("write-overwrite");
        let path = dir.join("config.json");
        atomic_write(&path, "lama", false).unwrap();
        atomic_write(&path, "baru", false).unwrap();
        assert_eq!(read_to_string(&path).unwrap(), "baru");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn strict_permissions_for_sensitive_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("write-perms");

        let pub_path = dir.join("models.json");
        atomic_write(&pub_path, "{}", false).unwrap();
        assert_eq!(
            std::fs::metadata(&pub_path).unwrap().permissions().mode() & 0o777,
            0o644
        );

        let secret = dir.join("auth.json");
        atomic_write(&secret, "{}", true).unwrap();
        assert_eq!(
            std::fs::metadata(&secret).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn reading_missing_file_returns_error() {
        let dir = temp_dir("write-missing");
        assert!(read_to_string(&dir.join("nope")).is_err());
    }
}
