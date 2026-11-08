//! Output untuk manusia: tabel, JSON, dan pratinjau diff.
//!
//! Semua pencetakan dilewatkan ke sini supaya `--json` konsisten di semua
//! perintah: satu tempat yang tahu kapan mencetak tabel dan kapan JSON.
use anyhow::Result;
use hakonfig_core::engine::model::{HarnessInfo, PreviewResult};

use crate::i18n::{self, Msg};

/// Batas baris diff per file. Provider hasil sync bisa punya ratusan model;
/// tanpa batas, terminal kebanjiran dan justru tidak terbaca.
const MAX_DIFF_LINES: usize = 40;

/// Daftar id harness yang didukung, untuk pesan error pemilihan harness.
/// Dipakai bersama oleh `main.rs` dan `add.rs`.
pub fn choices(harnesses: &[HarnessInfo]) -> String {
    i18n::fill(
        i18n::t(Msg::Choices),
        &[(
            "list",
            harnesses
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )],
    )
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let columns = headers.len();
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|header| header.chars().count())
        .collect();
    for row in &rows {
        for (index, cell) in row.iter().take(columns).enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    let render = |cells: &[String]| {
        let mut line = String::new();
        for (index, cell) in cells.iter().take(columns).enumerate() {
            if index + 1 == columns {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{:<width$}  ", cell, width = widths[index]));
            }
        }
        line.trim_end().to_string()
    };

    let header_cells: Vec<String> = headers.iter().map(|header| header.to_string()).collect();
    println!("{}", render(&header_cells));
    for row in rows {
        println!("{}", render(&row));
    }
}

/// Tampilkan pratinjau perubahan. Baris yang sama di awal/akhir tidak dicetak
/// supaya yang menonjol hanya bagian yang berubah.
pub fn print_preview(preview: &PreviewResult) {
    if preview.will_create.is_empty() && preview.will_modify.is_empty() {
        println!("{}", i18n::t(Msg::PreviewNoChanges));
        return;
    }

    for file in &preview.files {
        if !file.changed {
            continue;
        }
        let header = if preview.will_create.contains(&file.path) {
            Msg::PreviewNew
        } else {
            Msg::PreviewModified
        };
        println!(
            "\n{}",
            i18n::fill(i18n::t(header), &[("path", file.path.clone())])
        );
        if file.sensitive {
            println!("{}", i18n::t(Msg::PreviewSensitive));
        }
        print_line_diff(file.before.as_deref(), file.after.as_deref());
    }

    for warning in &preview.warnings {
        println!("\n! {warning}");
    }
}

fn print_line_diff(before: Option<&str>, after: Option<&str>) {
    let (removed, added) = changed_lines(before, after);
    let (removed_budget, added_budget) = split_budget(removed.len(), added.len(), MAX_DIFF_LINES);

    let mut omitted = removed.len() - removed_budget;
    for line in removed.iter().take(removed_budget) {
        println!("  - {line}");
    }
    omitted += added.len() - added_budget;
    for line in added.iter().take(added_budget) {
        println!("  + {line}");
    }

    if omitted > 0 {
        println!(
            "{}",
            i18n::fill(
                i18n::t(Msg::PreviewTruncated),
                &[("count", omitted.to_string())]
            )
        );
    }
}

/// Bagi kuota baris diff antara baris yang dihapus dan ditambah.
///
/// Kedua sisi dapat porsi setara, tapi sisa satu sisi (mis. hanya satu baris
/// yang dihapus) diberikan ke sisi lain, supaya baris baru — yang biasanya
/// paling penting dilihat — tidak terpotong hanya karena banyak baris lama
/// ikut berubah.
fn split_budget(removed: usize, added: usize, total: usize) -> (usize, usize) {
    let half = total / 2;
    if removed <= half {
        (removed, (total - removed).min(added))
    } else if added <= half {
        ((total - added).min(removed), added)
    } else {
        (half, half)
    }
}

/// Diff minimal: buang awalan/akhiran yang sama, sisakan bagian yang berubah.
fn changed_lines(before: Option<&str>, after: Option<&str>) -> (Vec<String>, Vec<String>) {
    let before: Vec<&str> = before.unwrap_or_default().lines().collect();
    let after: Vec<&str> = after.unwrap_or_default().lines().collect();

    let mut start = 0;
    while start < before.len() && start < after.len() && before[start] == after[start] {
        start += 1;
    }
    let mut tail = 0;
    while tail < before.len().saturating_sub(start)
        && tail < after.len().saturating_sub(start)
        && before[before.len() - 1 - tail] == after[after.len() - 1 - tail]
    {
        tail += 1;
    }

    let removed = before[start..before.len() - tail]
        .iter()
        .map(|line| line.to_string())
        .collect();
    let added = after[start..after.len() - tail]
        .iter()
        .map(|line| line.to_string())
        .collect();
    (removed, added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_shows_only_changed_lines() {
        let (removed, added) = changed_lines(
            Some("{\n  \"a\": 1,\n  \"b\": 2\n}"),
            Some("{\n  \"a\": 1,\n  \"b\": 3\n}"),
        );
        assert_eq!(removed, vec!["  \"b\": 2"]);
        assert_eq!(added, vec!["  \"b\": 3"]);
    }

    #[test]
    fn diff_addition_at_the_end() {
        let (removed, added) = changed_lines(Some("a\nb"), Some("a\nb\nc"));
        assert!(removed.is_empty(), "tidak ada yang dihapus: {removed:?}");
        assert_eq!(added, vec!["c"]);
    }

    #[test]
    fn diff_new_file_shows_everything() {
        let (removed, added) = changed_lines(None, Some("x\ny"));
        assert!(removed.is_empty());
        assert_eq!(added, vec!["x", "y"]);
    }

    #[test]
    fn diff_empty_file_does_not_panic() {
        let (removed, added) = changed_lines(None, None);
        assert!(removed.is_empty() && added.is_empty());
    }

    #[test]
    fn diff_identical_content_yields_nothing() {
        let (removed, added) = changed_lines(Some("a\nb"), Some("a\nb"));
        assert!(removed.is_empty() && added.is_empty());
    }

    #[test]
    fn diff_large_content_is_counted_for_truncation() {
        // 100 baris baru → 40 ditampilkan, 60 dilaporkan sebagai sisa.
        let big = (0..100)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (removed, added) = changed_lines(None, Some(&big));
        assert!(removed.is_empty());
        assert_eq!(added.len(), 100);
        assert_eq!(added.len() - MAX_DIFF_LINES, 60);
    }

    #[test]
    fn diff_budget_surplus_goes_to_the_other_side() {
        // 3 dihapus, 100 ditambah → yang ditambah tetap kebagian 37 baris.
        assert_eq!(split_budget(3, 100, MAX_DIFF_LINES), (3, 37));
        // 100 dihapus, 2 ditambah → yang dihapus kebagian 38 baris.
        assert_eq!(split_budget(100, 2, MAX_DIFF_LINES), (38, 2));
        // Kedua sisi besar → dibagi dua adil.
        assert_eq!(split_budget(50, 50, MAX_DIFF_LINES), (20, 20));
        // Tidak ada pemotongan sama sekali.
        assert_eq!(split_budget(5, 10, MAX_DIFF_LINES), (5, 10));
    }

    #[test]
    fn preview_without_changes_does_not_panic() {
        use hakonfig_core::engine::model::FileDiff;
        let preview = PreviewResult {
            files: vec![FileDiff {
                path: "/tmp/x".to_string(),
                role: "models".to_string(),
                sensitive: false,
                before: None,
                after: None,
                changed: false,
            }],
            warnings: Vec::new(),
            will_create: Vec::new(),
            will_modify: Vec::new(),
        };
        print_preview(&preview);
    }
}
