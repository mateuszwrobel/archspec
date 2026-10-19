pub mod capability;
pub mod cli;
pub mod commands;
pub mod config;
pub mod depgraph;
pub mod diagram;
pub mod helptext;
pub mod language;
pub mod model;
pub mod parse;
pub mod report;
pub mod scan;
pub mod spec;
pub mod update;
pub mod verify;

pub mod init;
pub mod inspect;

use cli::EX_USAGE;
use commands::registry::CommandInfo;
use commands::*;

pub(crate) const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "init",
        summary: "scaffold config + starter spec (no real-tree capture)",
        help: init::HELP,
    },
    CommandInfo {
        name: "scan",
        summary: "extract the architecture model only (no spec needed)",
        help: commands::scan::HELP,
    },
    CommandInfo {
        name: "diagram",
        summary: "render model (extracted or declared) to an artefact",
        help: commands::diagram::HELP,
    },
    CommandInfo {
        name: "verify",
        summary: "extract + compare vs spec, full diff, exit code",
        help: commands::verify::HELP,
    },
    CommandInfo {
        name: "update",
        summary: "seed a spec by snapshotting the real tree",
        help: commands::update::HELP,
    },
    CommandInfo {
        name: "report",
        summary: "text/markdown/json diff + metric output",
        help: commands::report::HELP,
    },
    CommandInfo {
        name: "spec",
        summary: "print the spec JSON schema or annotated reference",
        help: commands::spec::HELP,
    },
    CommandInfo {
        name: "doctor",
        summary: "diagnose which language drivers/toolchains are present",
        help: doctor::HELP,
    },
    CommandInfo {
        name: "inspect",
        summary: "zero-config file-level import map (discovery)",
        help: inspect::HELP,
    },
    CommandInfo {
        name: "depgraph",
        summary: "current-state module/submodule/API-usage views of a project",
        help: commands::depgraph::HELP,
    },
    CommandInfo {
        name: "help",
        summary: "print the archspec built-in manual (topics + per-command help)",
        help: commands::help::HELP,
    },
    CommandInfo {
        name: "capability",
        summary: "print the driver capability table (machine-readable guard surface): matrix | granular <language> <fact>",
        help: commands::capability::HELP,
    },
    CommandInfo {
        name: "skill",
        summary: "print or install the agent-facing audit skill",
        help: commands::skill::HELP,
    },
];

fn print_tool_help() {
    let mut out = String::from("archspec — multi-language architecture test & diagram tool\n\nusage: archspec <command> [args]\n\ncommands:\n");
    let width = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for cmd in COMMANDS {
        out.push_str(&format!("  {:width$}  {}\n", cmd.name, cmd.summary, width = width));
    }
    out.push_str("\nrun 'archspec <command> --help' for details on a command\n");
    out.push_str("run 'archspec --version' for the tool version\n");
    out.push_str("run 'archspec help' for topics: commands, glob, spec, constraints, languages, roles, workflow, capabilities, diagnostics\n");
    print!("{out}");
}

pub fn dispatch(args: &[String]) -> i32 {
    let Some(cmd) = args.first() else {
        print_tool_help();
        return EX_USAGE;
    };
    if cmd == "--help" {
        print_tool_help();
        return 0;
    }
    // Identity, not a command: "--version" (muscle memory) and "version"
    // (subcommand shape) both print the crate version. Unknown commands stay
    // refused below — this early return names exactly these two spellings.
    if cmd == "--version" || cmd == "version" {
        println!("archspec {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let rest = &args[1..];
    if rest.iter().any(|a| a == "--help")
        && let Some(info) = COMMANDS.iter().find(|c| c.name == cmd) {
            print!("{}", info.help);
            return 0;
        }
    let result = match cmd.as_str() {
        "inspect" => inspect::run(rest),
        "depgraph" => commands::depgraph::run(rest),
        "init" => init::run(rest),
        "scan" => commands::scan::run(rest),
        "diagram" => commands::diagram::run(rest),
        "verify" => commands::verify::run(rest),
        "update" => commands::update::run(rest),
        "report" => commands::report::run(rest),
        "spec" => commands::spec::run(rest),
        "help" => commands::help::run(rest, COMMANDS),
        "capability" => commands::capability::run(rest),
        "skill" => commands::skill::run(rest),
        "doctor" => doctor::run(rest),
        other => {
            eprintln!("error: unknown command: {other}");
            eprintln!("run 'archspec --help' for the command list");
            return EX_USAGE;
        }
    };
    match result {
        Ok(()) => 0,
        Err(msg) => {
            if msg == commands::verify::RULE_VIOLATION_REPORTED {
                // The report was already printed to stdout; do not echo it to
                // stderr as an error message.
                if cmd == "verify" {
                    eprintln!("note: archspec {}", env!("CARGO_PKG_VERSION"));
                }
                return 1;
            }
            // The exit-taxonomy classes travel on the message as sentinel
            // tags (cli::usage_error / cli::operational_error): dispatch
            // strips the tag and prints the bare message exactly as untagged
            // errors print it, so error text stays byte-identical while the
            // code states the class.
            if let Some((code, msg)) = cli::classify(&msg) {
                eprintln!("error: {msg}");
                // The footer states the binary behind a verdict-class exit;
                // usage errors (64) are CLI misuse with byte-frozen refusal
                // text, so they stay footer-free (exit-taxonomy bytes law).
                if cmd == "verify" && code != EX_USAGE {
                    eprintln!("note: archspec {}", env!("CARGO_PKG_VERSION"));
                }
                return code;
            }
            eprintln!("error: {msg}");
            if cmd == "verify" {
                eprintln!("note: archspec {}", env!("CARGO_PKG_VERSION"));
            }
            1
        }
    }
}
