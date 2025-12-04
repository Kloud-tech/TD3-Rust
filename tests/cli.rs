use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn make_log_file() -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temp file");
    write!(
        file,
        "\
2024-01-15 10:30:45 [INFO] Application started
2024-01-15 10:31:15 [ERROR] Failed to connect to API: timeout
2024-01-15 10:32:00 [ERROR] Database query failed: syntax error
2024-01-15 11:00:00 [INFO] Done
"
    )
    .unwrap();
    file
}

#[test]
fn shows_help() {
    cargo_bin_cmd!("TD3-Rust")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn filters_errors_and_search() {
    let file = make_log_file();
    cargo_bin_cmd!("TD3-Rust")
        .arg("--errors-only")
        .arg("--search")
        .arg("database")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Database query failed"));
}

#[test]
fn respects_since_until_filters() {
    let file = make_log_file();
    cargo_bin_cmd!("TD3-Rust")
        .arg("--since")
        .arg("2024-01-15 10:31:30")
        .arg("--until")
        .arg("2024-01-15 10:31:50")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Aucune entrée"));
}

#[test]
fn fails_on_top_zero() {
    cargo_bin_cmd!("TD3-Rust")
        .arg("--top")
        .arg("0")
        .arg("missing.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "La valeur de --top doit être au moins 1",
        ));
}

#[test]
fn reports_missing_file() {
    cargo_bin_cmd!("TD3-Rust")
        .arg("definitely_missing.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Fichier introuvable"));
}
