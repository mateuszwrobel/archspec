use std::collections::{BTreeMap, BTreeSet};

/// Exit code for the usage class: no command, unknown command, unknown flag,
/// bad or missing flag value, `help` typo. Universal across the tool's
/// surface — an argument problem is never a verdict.
pub const EX_USAGE: i32 = 64;
/// Exit code for the operational-precondition class: the spec file a verb
/// needs is missing. A missing spec is never answered as a verdict.
pub const EX_OPERATIONAL: i32 = 70;

/// Sentinel prefixes on `Err` payloads that carry a class to dispatch: a tag
/// byte a real message never carries. See `classify` and the two constructors
/// below; the bare message prints unchanged, only the code states the class.
const USAGE_TAG: &str = "\u{1}archspec-usage:";
const OPERATIONAL_TAG: &str = "\u{1}archspec-operational:";

/// Wrap a usage-class message (exit 64): bad arguments, unknown verb,
/// unknown flag, `help` typo.
pub fn usage_error(msg: impl Into<String>) -> String {
    format!("{USAGE_TAG}{}", msg.into())
}

/// Wrap an operational-precondition message (exit 70): the spec/config the
/// verb needs is missing.
pub fn operational_error(msg: impl Into<String>) -> String {
    format!("{OPERATIONAL_TAG}{}", msg.into())
}

/// Classify an `Err` payload: `Some((exit code, bare message))` when a class
/// tag is present, `None` for the verdict and everything-else class that
/// keeps the historical exit 1.
pub fn classify(msg: &str) -> Option<(i32, &str)> {
    if let Some(bare) = msg.strip_prefix(USAGE_TAG) {
        Some((EX_USAGE, bare))
    } else if let Some(bare) = msg.strip_prefix(OPERATIONAL_TAG) {
        Some((EX_OPERATIONAL, bare))
    } else {
        None
    }
}

pub struct Args {
    pub positionals: Vec<String>,
    pub values: BTreeMap<String, String>,
    pub switches: BTreeSet<String>,
}

pub struct FlagSpec {
    pub name: &'static str,
    pub takes_value: bool,
}

pub fn parse(args: &[String], specs: &[FlagSpec]) -> Result<Args, String> {
    let mut positionals = Vec::new();
    let mut values = BTreeMap::new();
    let mut switches = BTreeSet::new();

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if let Some(name) = arg.strip_prefix("--") {
            let spec = specs
                .iter()
                .find(|s| s.name == name)
                .ok_or_else(|| usage_error(format!("unknown flag: --{name}")))?;
            if spec.takes_value {
                let value = it
                    .next()
                    .ok_or_else(|| usage_error(format!("flag --{name} requires a value")))?;
                values.insert(name.to_string(), value.clone());
            } else {
                switches.insert(name.to_string());
            }
        } else {
            positionals.push(arg.clone());
        }
    }

    Ok(Args {
        positionals,
        values,
        switches,
    })
}

pub fn exactly_one_positional(args: &Args) -> Result<(), String> {
    if args.positionals.len() > 1 {
        Err("expected at most one path argument".to_string())
    } else {
        Ok(())
    }
}
