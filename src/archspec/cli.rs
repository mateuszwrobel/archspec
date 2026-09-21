use std::collections::{BTreeMap, BTreeSet};

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
                .ok_or_else(|| format!("unknown flag: --{name}"))?;
            if spec.takes_value {
                let value = it
                    .next()
                    .ok_or_else(|| format!("flag --{name} requires a value"))?;
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
