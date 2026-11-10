mod archspec;

// Phase-profiling harness gate (issue #34, operator ruling): the shipped
// binary carries zero profiling cost. `profile-phases` is a dev-only cargo
// feature whose sole consumer is the `phase-probe` example; a binary build
// that passes it must fail loud here rather than bake instrumentation into
// a release artifact. Test builds are exempt so the gate can compile the
// suite with the feature; the guards in tests/profile_gate.rs pin the shape.
#[cfg(all(feature = "profile-phases", not(test)))]
compile_error!(
    "`profile-phases` is a dev-only harness feature: build the phase-probe example \
     (`cargo run --example phase-probe --features profile-phases`) instead of the binary"
);

// musl allocator gate (issue #33): the parallel fact leg allocates on every
// worker, and musl's default allocator serializes those allocations on a
// global lock — under musl the parallel leg is 2-3x SLOWER than serial while
// glibc scales. mimalloc's thread-local heaps remove the convoy, and the
// dependency's `override` feature defines the malloc-family symbols too:
// the fact leg's tree-sitter parses allocate through the C library malloc,
// which a Rust global_allocator alone cannot reach (measured: without
// `override` the inversion persisted; with it the ratio moved 1.67 -> 0.67
// on the pinned 20k-file corpus). The gate is double: the feature must be
// enabled AND the target env must be musl, so non-musl targets and every
// dev/test build keep the system allocator and a byte-unchanged dependency
// graph. The release matrix enables `musl-alloc` in the musl build steps
// only; tests link the library, never this binary, so test builds never see
// the allocator at all.
#[cfg(all(feature = "musl-alloc", target_env = "musl"))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(unix)]
unsafe extern "C" {
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
