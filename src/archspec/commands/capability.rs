//! `archspec capability` — machine-readable projection of the driver
//! capability table (the single source of truth in `capability.rs`). This is
//! the surface prose-drift guards consult: parsing the printed matrix rows is
//! equivalent to reading the table, so no test maintains a second list of
//! what each driver emits. Output is deterministic and append-only in shape.
//!
//! * `archspec capability matrix` — one row per line:
//!   `capability <language> <fact> <emission>` plus `rule <rule> <fact>`
//!   lines for the checks that consume a fact.
//! * `archspec capability granular <language> <fact>` — exit 0 when the row
//!   states full-granularity emission, exit 1 otherwise (and on an unknown
//!   pair); prints nothing on success.

use crate::archspec::capability;
use crate::archspec::cli;
use crate::archspec::language;

pub const HELP: &str = "\
usage: archspec capability matrix
       archspec capability granular <language> <fact>

matrix   prints the driver capability table verbatim:
           capability <language> <fact> <emission>
           rule <rule> <fact>
granular exits 0 when the driver emits the fact at full granularity,
         1 otherwise; the machine query behind the prose-drift guards.
";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(args, &[])?;
    let mut positionals = parsed.positionals.iter().map(String::as_str);
    match positionals.next() {
        Some("matrix") => {
            if positionals.next().is_some() {
                return Err("capability matrix takes no arguments".to_string());
            }
            for (language, fact, emission) in capability::CAPABILITIES {
                println!("capability {} {fact} {emission}", language.as_str());
            }
            for (rule, fact) in capability::RULES {
                println!("rule {rule} {fact}");
            }
            Ok(())
        }
        Some("granular") => {
            let language = positionals
                .next()
                .ok_or_else(|| "usage: archspec capability granular <language> <fact>".to_string())?;
            let fact = positionals
                .next()
                .ok_or_else(|| "usage: archspec capability granular <language> <fact>".to_string())?;
            if positionals.next().is_some() {
                return Err("capability granular takes two arguments".to_string());
            }
            let language = language::from_name(language)
                .ok_or_else(|| format!("unknown language: {language}"))?;
            if capability::emission(language, fact).is_none() {
                return Err(format!("unknown fact for {}: {fact}", language.as_str()));
            }
            if capability::emits_granular(language, fact) {
                Ok(())
            } else {
                Err("not-granular".to_string())
            }
        }
        _ => Err("usage: archspec capability matrix | granular <language> <fact>".to_string()),
    }
}
