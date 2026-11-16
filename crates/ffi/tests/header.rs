//! `include/mini_games.h` is written by hand; these checks keep it in step with the exports.

use std::collections::BTreeSet;
use std::path::Path;

use mini_games::{MG_ABI_VERSION, MgStatus};

const HEADER: &str = include_str!("../include/mini_games.h");

/// Every `mg_*` identifier directly followed by `(`.
fn called_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("mg_") {
        let tail = &rest[start..];
        let len = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(tail.len());
        if tail[len..].starts_with('(') {
            names.insert(tail[..len].to_owned());
        }
        rest = &tail[len..];
    }
    names
}

fn exported_names() -> BTreeSet<String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(src).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            if line.trim() != "#[unsafe(no_mangle)]" {
                continue;
            }
            let signature = lines.next().unwrap();
            let name = signature
                .split("fn ")
                .nth(1)
                .and_then(|rest| rest.split('(').next())
                .unwrap();
            names.insert(name.to_owned());
        }
    }
    names
}

#[test]
fn header_declares_exactly_the_exported_functions() {
    let exported = exported_names();
    assert!(exported.len() > 30, "{exported:?}");
    assert_eq!(called_names(HEADER), exported);
}

fn header_constant(name: &str) -> i64 {
    let prefix = format!("#define {name} ");
    let line = HEADER
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("{name} is not defined in the header"));
    line[prefix.len()..].trim().parse().unwrap()
}

fn header_status_name(status: MgStatus) -> &'static str {
    match status {
        MgStatus::Ok => "MG_STATUS_OK",
        MgStatus::NullArgument => "MG_STATUS_NULL_ARGUMENT",
        MgStatus::InvalidArgument => "MG_STATUS_INVALID_ARGUMENT",
        MgStatus::Rejected => "MG_STATUS_REJECTED",
        MgStatus::Desync => "MG_STATUS_DESYNC",
        MgStatus::Internal => "MG_STATUS_INTERNAL",
        MgStatus::InvalidState => "MG_STATUS_INVALID_STATE",
    }
}

#[test]
fn header_constants_match_the_library() {
    let statuses = [
        MgStatus::Ok,
        MgStatus::NullArgument,
        MgStatus::InvalidArgument,
        MgStatus::Rejected,
        MgStatus::Desync,
        MgStatus::Internal,
        MgStatus::InvalidState,
    ];
    for status in statuses {
        assert_eq!(
            header_constant(header_status_name(status)),
            i64::from(status as i32)
        );
    }
    let defined = HEADER
        .lines()
        .filter(|line| line.starts_with("#define MG_STATUS_"))
        .count();
    assert_eq!(defined, statuses.len());
    assert_eq!(header_constant("MG_ABI_VERSION"), i64::from(MG_ABI_VERSION));
}
