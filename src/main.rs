mod archspec;

#[cfg(unix)]
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Rust installs SIG_IGN for SIGPIPE at startup, so a write to a closed pipe
/// surfaces as an io::Error and the print! macros panic. Restoring the system
/// default disposition at the CLI entry makes `archspec ... | head -1` end the
/// process by signal (128+13 = 141 through the shell) with no panic text —
/// the standard CLI convention for a closed consumer.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe { signal(SIGPIPE, SIG_DFL) };
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = archspec::dispatch(&args);
    std::process::exit(code);
}
