pub mod capability;
pub mod cli;
pub mod commands;
pub mod config;
pub mod depgraph;
pub mod diagram;
pub mod language;
pub mod model;
pub mod report;
pub mod scan;
pub mod spec;
pub mod update;
pub mod verify;

pub mod init;
pub mod inspect;

use commands::*;

pub struct CommandInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub help: &'static str,
}

pub(crate) const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "init",
        summary: "scaffold a minimal base spec",
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
        summary: "snapshot current model as a seed spec",
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
        summary: "print the driver capability table (machine-readable guard surface)",
        help: commands::capability::HELP,
    },
];

fn print_tool_help() {
    let mut out = String::from("archspec — multi-language architecture test & diagram tool\n\nusage: archspec <command> [args]\n\ncommands:\n");
    let width = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for cmd in COMMANDS {
        out.push_str(&format!("  {:width$}  {}\n", cmd.name, cmd.summary, width = width));
    }
    out.push_str("\nrun 'archspec <command> --help' for details on a command\n");
    out.push_str("run 'archspec help' for topics: commands, glob, spec, constraints, languages, workflow, diagnostics\n");
    print!("{out}");
}

pub fn dispatch(args: &[String]) -> i32 {
    let Some(cmd) = args.first() else {
        print_tool_help();
        return 0;
    };
    if cmd == "--help" {
        print_tool_help();
        return 0;
    }
    let rest = &args[1..];
    if rest.iter().any(|a| a == "--help") {
        if let Some(info) = COMMANDS.iter().find(|c| c.name == cmd) {
            print!("{}", info.help);
            return 0;
        }
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
        "help" => commands::help::run(rest),
        "capability" => commands::capability::run(rest),
        "doctor" => doctor::run(rest),
        other => {
            eprintln!("error: unknown command: {other}");
            eprintln!("run 'archspec --help' for the command list");
            return 1;
        }
    };
    match result {
        Ok(()) => 0,
        Err(msg) => {
            if msg == commands::verify::RULE_VIOLATION_REPORTED {
                // The report was already printed to stdout; do not echo it to
                // stderr as an error message.
                return 1;
            }
            eprintln!("error: {msg}");
            1
        }
    }
}
