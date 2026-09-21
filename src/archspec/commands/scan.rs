use crate::archspec::scan;

pub const HELP: &str = "usage: archspec scan [path] [--output <path>] [--check]\nextract the architecture model (units, tiers, external) from a project tree\n\n  path          project directory to scan (default: current directory)\n  --output <p>  write the model JSON to that file (default: stdout unless archspec.toml [output] sets a destination)\n  --check       compare the model to its destination without writing; exit non-zero if stale or missing\n";

pub fn run(args: &[String]) -> Result<(), String> {
    scan::run(args)
}
