#![allow(dead_code)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

pub struct Fixture {
    _temp: TempDir,
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().to_path_buf();
        Self { _temp: temp, root }
    }

    pub fn write(&self, relative_path: &str, content: &str) {
        write_file(&self.root, relative_path, content);
    }

    pub fn path(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path)
    }

    pub fn run(&self, args: &[&str]) -> Output {
        run_in(&self.root, args)
    }

    pub fn run_in(&self, dir: &str, args: &[&str]) -> Output {
        run_in(&self.root.join(dir), args)
    }

    pub fn read(&self, relative_path: &str) -> String {
        fs::read_to_string(self.root.join(relative_path)).expect("read fixture file")
    }
}

pub fn write_file(root: &Path, relative_path: &str, content: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent dir");
    }
    fs::write(path, content).expect("write file");
}

pub fn run_in(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run archspec")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Declaration-order property for mermaid model renders (audit defect D3):
/// every node id referenced by an arrow (` --> `) must appear on a
/// declaration line ABOVE the arrow — a node line (`id` or `id["label"]`) or
/// a subgraph header (`subgraph id` / `subgraph id["title"]`). An id
/// referenced before any declaration renders as a detached, unlabelled node
/// disconnected from its subgraphs, so no model-tier renderer may emit one.
pub fn assert_declared_before_reference(label: &str, text: &str) {
    let mut declared = std::collections::BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains(" --> ") {
            for reference in trimmed.split(" --> ") {
                let id = reference.trim();
                assert!(
                    declared.contains(id),
                    "{label}: id {id:?} on line {} is referenced before any \
                     declaration line:\n{text}",
                    index + 1
                );
            }
        } else if trimmed == "end" || trimmed.starts_with("graph ") {
            // Structural keywords: neither declarations nor references.
        } else if let Some(header) = trimmed.strip_prefix("subgraph ") {
            declared.insert(declared_id(header.trim()));
        } else {
            declared.insert(declared_id(trimmed));
        }
    }
}

/// The id part of a declaration line: everything before the quoted label.
fn declared_id(declaration: &str) -> String {
    match declaration.split_once('[') {
        Some((id, _)) => id.trim().to_string(),
        None => declaration.to_string(),
    }
}
