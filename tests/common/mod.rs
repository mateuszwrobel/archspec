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
