//! Home harness milik user lain (ADE / VPS).
//!
//! Setup agentic sering memberi tiap agent user dan home sendiri (service
//! systemd dengan `User=openbot`, home `/var/lib/openbot`). Modul ini:
//!
//! * [`list_targets`] memindai `/etc/passwd` untuk home di luar pola
//!   `/home/<user>` dan menandai harness yang terdeteksi di sana;
//! * [`resolve_target`] menerima `<user>` atau `<dir>` untuk `--target`;
//! * [`cross_home_notices`] dan [`ownership_notices`] menghasilkan notice
//!   doctor yang menjelaskan gejala "sudah di-add tapi tidak terbaca".
//!
//! Semua fungsi murni menerima data yang bisa dibuat di test; pembacaan
//! `/etc/passwd` diisolasi di [`passwd_entries`].
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::model::Notice;
use super::paths;

/// Harness terdeteksi di home milik satu user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub user: String,
    pub home: String,
    /// id harness yang config-nya atau binary-nya ada di home ini.
    pub harnesses: Vec<String>,
}

/// Satu entri `/etc/passwd` yang relevan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswdEntry {
    pub user: String,
    pub home: PathBuf,
}

/// Harness yang jejaknya ada di `home`: config asli atau binary di
/// `<home>/.local/bin` (diambil dari katalog). Urut alfabetis, tanpa duplikat.
pub fn harnesses_in(home: &Path) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    let config_markers: &[(&str, &[&str])] = &[
        ("omp", &[".omp/agent/models.yml", ".omp/agent/models.yaml"]),
        ("pi", &[".pi/agent/models.json"]),
        ("hermes", &[".hermes/config.yaml"]),
        (
            "opencode",
            &[
                ".config/opencode/opencode.json",
                ".config/opencode/opencode.jsonc",
            ],
        ),
    ];
    for (id, markers) in config_markers {
        if markers.iter().any(|marker| home.join(marker).exists()) {
            found.push((*id).to_string());
        }
    }

    // Binary di .local/bin: menutup kasus harness terpasang yang belum pernah
    // membuat config (justru kondisi home ADE yang baru).
    for entry in super::catalog::all() {
        let Some(binary) = entry.binary.as_deref().filter(|b| !b.trim().is_empty()) else {
            continue;
        };
        if home.join(".local/bin").join(binary).exists() && !found.iter().any(|id| id == &entry.id)
        {
            found.push(entry.id.to_string());
        }
    }

    found.sort();
    found.dedup();
    found
}

/// Baca dan parse `/etc/passwd`. Entri dengan home kosong atau `/` di-skip.
pub fn passwd_entries() -> Vec<PasswdEntry> {
    let Ok(text) = std::fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    parse_passwd(&text)
}

/// Parse isi `/etc/passwd` (`name:x:uid:gid:gecos:home:shell`).
pub fn parse_passwd(text: &str) -> Vec<PasswdEntry> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() < 7 {
                return None;
            }
            let home = PathBuf::from(fields[5]);
            if !home.is_absolute() || home == Path::new("/") {
                return None;
            }
            Some(PasswdEntry {
                user: fields[0].to_string(),
                home,
            })
        })
        .collect()
}

/// Home di luar pola `/home/<user>`: calon home ADE. Menutup kasus user
/// sistem (uid < 1000) yang home-nya di /var/lib, jadi tidak difilter uid.
pub fn is_agent_like_home(home: &Path) -> bool {
    let Some(plain) = home.to_str() else {
        return false;
    };
    let Some(rest) = plain.strip_prefix("/home/") else {
        return true;
    };
    // /home/<name> pola biasa; /home/<name>/... (subdir) bukan home user.
    rest.contains('/') || rest.is_empty()
}

/// Semua home agent lain yang harness-nya terdeteksi, di luar `current`.
pub fn list_targets() -> Vec<TargetInfo> {
    let current = paths::target_home();
    passwd_entries()
        .into_iter()
        .filter(|entry| is_agent_like_home(&entry.home) && entry.home != current)
        .map(|entry| {
            let harnesses = harnesses_in(&entry.home);
            (entry, harnesses)
        })
        .filter(|(_, harnesses)| !harnesses.is_empty())
        .map(|(entry, harnesses)| TargetInfo {
            user: entry.user,
            home: entry.home.display().to_string(),
            harnesses,
        })
        .collect()
}

/// `--target <user|dir>`: username dikenali dari `/etc/passwd` → home-nya;
/// selain itu dianggap path direktori home.
pub fn resolve_target(spec: &str) -> Option<PathBuf> {
    let spec = spec.trim();
    if let Some(entry) = passwd_entries()
        .into_iter()
        .find(|entry| entry.user == spec)
    {
        return Some(entry.home);
    }
    let path = PathBuf::from(spec);
    if path.is_absolute() && path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// Notice `ade_home_mismatch`: harness terdeteksi di home user lain, sehingga
/// pengelolaan dari HOME proses ini tidak pernah sampai ke agent-nya.
///
/// `entries` dipisah supaya bisa diuji tanpa menyentuh `/etc/passwd`.
pub fn cross_home_notices(
    harness_ids: &[String],
    current_home: &Path,
    entries: &[PasswdEntry],
) -> Vec<Notice> {
    let mut notices = Vec::new();
    for entry in entries {
        if entry.home == current_home || !is_agent_like_home(&entry.home) {
            continue;
        }
        for id in harness_ids {
            if harnesses_in(&entry.home).iter().any(|found| found == id) {
                notices.push(Notice {
                    code: "ade_home_mismatch".to_string(),
                    harness: id.clone(),
                    provider: entry.user.clone(),
                    message: format!(
                        "'{id}' is also installed for user '{user}' (home {home}); \
                         this run manages {current}, so changes never reach that agent. \
                         Target it with: hakonfig-cli --home {home} <command>",
                        user = entry.user,
                        home = entry.home.display(),
                        current = current_home.display(),
                    ),
                    fix: None,
                });
            }
        }
    }
    notices
}

/// Notice `ownership_mismatch`: file config dimiliki uid lain daripada pemilik
/// home. Gejala khas "sudah di-add tapi agent tidak membaca apa pun" — biasanya
/// bekas penulisan sebagai root. Hakonfig tidak pernah chown otomatis.
///
/// `home_uid` adalah uid pemilik home target; `path_uid` memetakan path ke
/// uid pemiliknya (dipisah supaya bisa diuji tanpa filesystem nyata).
pub fn ownership_notices(
    harness_id: &str,
    files: &[PathBuf],
    home_uid: u32,
    path_uid: impl Fn(&Path) -> Option<u32>,
) -> Vec<Notice> {
    // `path_uid` yang bertanggung jawab mengembalikan None untuk path yang
    // tidak ada, jadi fungsi ini tetap bisa diuji tanpa filesystem nyata.
    files
        .iter()
        .filter_map(|path| {
            let owner = path_uid(path)?;
            if owner == home_uid {
                return None;
            }
            Some(Notice {
                code: "ownership_mismatch".to_string(),
                harness: harness_id.to_string(),
                provider: String::new(),
                message: format!(
                    "{} is owned by uid {} but the home belongs to uid {}; the harness \
                     may not be able to read its own config. Fix manually: \
                     sudo chown -R {}:{} {} (Hakonfig never changes ownership automatically)",
                    path.display(),
                    owner,
                    home_uid,
                    home_uid,
                    home_uid,
                    path.display(),
                ),
                fix: None,
            })
        })
        .collect()
}

#[cfg(unix)]
pub fn file_owner_uid(path: &Path) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|meta| meta.uid())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_home(home: &Path, relative: &str) {
        let path = home.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "x").unwrap();
    }

    #[test]
    fn harnesses_in_finds_config_and_binaries() {
        let home = std::env::temp_dir().join(format!("hkt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        write_home(&home, ".omp/agent/models.yml");
        write_home(&home, ".local/bin/omp");
        write_home(&home, ".pi/agent/models.json");
        write_home(&home, ".config/opencode/opencode.json");

        let mut found = harnesses_in(&home);
        found.sort();
        assert_eq!(found, vec!["omp", "opencode", "pi"]);

        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn parse_passwd_skips_invalid_and_root_entries() {
        let entries = parse_passwd(
            "root:x:0:0:root:/root:/bin/bash\n\
             openbot:x:998:998::/var/lib/openbot:/usr/sbin/nologin\n\
             broken:x:1:1\n\
             odd:x:0:0:root:/:/bin/sh\n",
        );
        assert_eq!(entries.len(), 2, "root (/root) dan entri rusak di-skip");
        assert_eq!(entries[0].user, "root");
        assert_eq!(entries[0].home, PathBuf::from("/root"));
        assert_eq!(entries[1].user, "openbot");
    }

    #[test]
    fn agent_like_home_excludes_the_regular_home_pattern() {
        assert!(is_agent_like_home(Path::new("/var/lib/openbot")));
        assert!(is_agent_like_home(Path::new("/root")));
        assert!(!is_agent_like_home(Path::new("/home/hudya")));
        // "/" tidak pernah sampai sini (parse_passwd men-skip), tapi bila
        // dipanggil langsung diperlakukan seperti home agent, bukan crash.
        assert!(is_agent_like_home(Path::new("/")));
    }

    #[test]
    fn cross_home_notice_points_at_the_right_home() {
        let current = Path::new("/home/ubuntu");
        let empty: Vec<PasswdEntry> = Vec::new();
        assert!(
            cross_home_notices(&["omp".to_string()], current, &empty).is_empty(),
            "tanpa entri tidak ada notice"
        );

        let home = std::env::temp_dir().join(format!("hkn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        write_home(&home, ".omp/agent/models.yml");
        let entries = vec![PasswdEntry {
            user: "openbot".to_string(),
            home: home.clone(),
        }];

        let notices = cross_home_notices(&["omp".to_string()], current, &entries);
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].code, "ade_home_mismatch");
        assert_eq!(notices[0].harness, "omp");
        assert!(
            notices[0].message.contains("--home"),
            "{:?}",
            notices[0].message
        );
        assert!(notices[0]
            .message
            .contains(home.display().to_string().as_str()));
        assert!(notices[0].fix.is_none(), "perbaikan manual, bukan otomatis");

        std::fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn ownership_notice_fires_when_uid_differs() {
        let files = vec![PathBuf::from("/var/lib/openbot/.omp/agent/models.yml")];
        let notices = ownership_notices("omp", &files, 1000, |path| {
            // Simulasi: file dimiliki root, home milik uid 998.
            if path == files[0] {
                Some(0)
            } else {
                None
            }
        });
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].code, "ownership_mismatch");
        assert!(
            notices[0].message.contains("chown"),
            "{:?}",
            notices[0].message
        );

        let same = ownership_notices("omp", &files, 0, |_| Some(0));
        assert!(same.is_empty(), "uid sama tidak boleh notice");
    }
}
