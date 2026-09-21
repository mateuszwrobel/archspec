use std::path::Path;

use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    Rust,
    Csharp,
    Go,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Csharp => "csharp",
            Language::Go => "go",
        }
    }
}

/// Parse a spec/model language name into a driver language. The mirror of
/// `as_str`; unknown names yield `None` (consumers then fall back to the
/// granular assumption rather than guessing a driver).
pub fn from_name(value: &str) -> Option<Language> {
    match value {
        "rust" => Some(Language::Rust),
        "csharp" => Some(Language::Csharp),
        "go" => Some(Language::Go),
        _ => None,
    }
}

/// True if the tree contains at least one source file in the given language.
/// Used by spec-driven consumers (`report`) to reject a tree with no sources in
/// the spec's declared language before extraction (commands/report/errors.md).
pub fn has_sources(root: &Path, language: Language) -> bool {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            if !entry.file_type().is_file() {
                return false;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            match language {
                Language::Rust => name == "Cargo.toml" || name.ends_with(".rs"),
                Language::Csharp => {
                    name.ends_with(".csproj") || name.ends_with(".sln") || name.ends_with(".cs")
                }
                Language::Go => name == "go.mod" || name.ends_with(".go"),
            }
        })
}

pub fn detect(root: &Path) -> Option<Language> {
    let mut has_rust = false;
    let mut has_csharp = false;
    let mut has_go = false;

    for entry in WalkDir::new(root) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "Cargo.toml" || name.ends_with(".rs") {
            has_rust = true;
        } else if name.ends_with(".csproj") || name.ends_with(".sln") || name.ends_with(".cs") {
            has_csharp = true;
        } else if name == "go.mod" || name.ends_with(".go") {
            has_go = true;
        }
    }

    if has_rust {
        Some(Language::Rust)
    } else if has_csharp {
        Some(Language::Csharp)
    } else if has_go {
        Some(Language::Go)
    } else {
        None
    }
}
