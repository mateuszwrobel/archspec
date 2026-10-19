/// Metadata about one command: its name, one-line purpose and `--help` text.
///
/// This leaf owns the type shared by the command registry table in the crate
/// root and the help surface. It imports nothing, so no consumer of the
/// registry can close a cycle through it.
pub struct CommandInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub help: &'static str,
}
