//! Penulisan file yang aman (atomik + permission).
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Tulis ke file sementara di direktori yang sama, lalu `rename`.
/// Ini mencegah file konfigurasi berakhir setengah tertulis bila proses mati.
pub fn atomic_write(path: &Path, contents: &str, sensitive: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("gagal membuat direktori {}", parent.display()))?;
    }

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    let tmp = path.with_file_name(format!(".{}.rekonfig-tmp", file_name));

    {
        let mut file = std::fs::File::create(&tmp)
            .with_context(|| format!("gagal menulis {}", tmp.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("gagal menulis {}", tmp.display()))?;
        file.sync_all().ok();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if sensitive { 0o600 } else { 0o644 };
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
        }
    }

    std::fs::rename(&tmp, path).with_context(|| format!("gagal mengganti {}", path.display()))?;
    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("gagal membaca {}", path.display()))
}
