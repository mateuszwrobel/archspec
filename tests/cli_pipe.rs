#![cfg(unix)]

mod common;

use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Run one render command and close its stdout pipe once the first byte is
/// in flight — the `archspec ... | head -1` condition: the consumer exits
/// before the writer is done, so the next write hits a closed pipe.
fn closed_pipe_render(args: &[&str], cwd: &Path) -> (Option<i32>, Option<i32>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn archspec");
    let mut pipe = child.stdout.take().expect("piped stdout");
    let mut first = [0u8; 1];
    let _ = pipe.read(&mut first);
    drop(pipe);
    let output = child.wait_with_output().expect("wait archspec");
    (
        output.status.code(),
        output.status.signal(),
        common::stderr(&output),
    )
}

/// Scenario: closed stdout is silence, not panic. Given a diagram/report/scan
/// render piped to a consumer that exits first, when the writer hits the
/// closed pipe, the process terminates without panic text and with a non-crash
/// status: a quiet death by SIGPIPE (signal 13, status 141 through a shell) or
/// 0 when the output had already flushed before the pipe closed.
#[test]
fn closed_pipe_render_is_silence_not_panic() {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"pipes\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn lib() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"pipes\"\nmatches = { units = [\"pipes\"] }\n",
    );

    for cmd in [
        vec!["scan"],
        vec!["diagram"],
        vec!["report"],
        vec!["inspect"],
    ] {
        let (code, signal, err) = closed_pipe_render(&cmd, &fixture.root);
        assert!(
            !err.contains("panicked"),
            "{cmd:?} must not panic on a closed pipe:\n{err}"
        );
        assert!(
            signal == Some(13) || code == Some(0),
            "{cmd:?} must die quietly by SIGPIPE (13) or exit 0, got code={code:?} signal={signal:?} stderr:\n{err}"
        );
    }
}
