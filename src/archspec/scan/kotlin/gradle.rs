//! Gradle build-script and version-catalog evidence (kotlin driver record
//! K-P2/K-P4): archspec executes nothing (the no-gradelaw mirrors the python
//! no-venv law), so the dependency evidence is read statically from the
//! module's build script — inline coordinate literals plus `libs.*` catalog
//! accessors — and from `gradle/libs.versions.toml`.
//!
//! The test-tier evidence half is the junit FAMILY (K-P4): GROUP or NAME
//! matching `junit*` / `org.junit*` — the bare name `junit` alone is not the
//! predicate (JUnit 5 declares `org.junit.*` and reads `junit` nowhere, the
//! `pytest*` precedent) — and it must sit in a TEST configuration (a
//! configuration name containing "test"). A catalog that cannot be parsed
//! degrades the alias evidence to NOTHING plus the report-only line
//! `gradle catalog unreadable: <path> — test-tier evidence unavailable`
//! (inline coordinates keep their evidence — the line names what is lost).

use std::collections::BTreeSet;
use std::path::Path;

/// The dependency configurations the static reader recognizes (the
/// java/kotlin plugin family). Names are matched case-insensitively.
const DEPENDENCY_CONFIGURATIONS: &[&str] = &[
    "api",
    "implementation",
    "compileonly",
    "runtimeonly",
    "kapt",
    "annotationprocessor",
    "testapi",
    "testimplementation",
    "testcompileonly",
    "testruntimeonly",
    "androidtestapi",
    "androidtestimplementation",
    "androidtestcompileonly",
    "androidtestruntimeonly",
    "debugimplementation",
    "releaseimplementation",
    "corelibrarygroups",
];

/// One dependency a build script states.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Dependency {
    /// `implementation("group:artifact:version")` — the inline GAV shape.
    Coordinate {
        group: String,
        artifact: String,
        configuration: String,
    },
    /// `implementation(project(":core"))` — a first-party module dependency.
    Project { path: String },
    /// `testImplementation(libs.junit)` — the catalog accessor half; the
    /// resolved group/name, or the alias text when the catalog is offline
    /// or the alias misses (the degrade arm, K-P4).
    Catalog {
        alias: String,
        group: Option<String>,
        name: Option<String>,
        configuration: String,
    },
}

impl Dependency {
    /// True when this row is a junit-FAMILY dependency in a TEST
    /// configuration (the K-P4 evidence half; family = group or name
    /// starts_with `junit` / `org.junit`, case-insensitive).
    pub(crate) fn junit_evidence(&self) -> bool {
        let configuration = match self {
            Dependency::Coordinate { configuration, .. }
            | Dependency::Catalog { configuration, .. } => configuration,
            Dependency::Project { .. } => return false,
        };
        if !configuration.to_lowercase().contains("test") {
            return false;
        }
        let (group, name) = match self {
            Dependency::Coordinate { group, artifact, .. } => (group.as_str(), artifact.as_str()),
            Dependency::Catalog { group, name, .. } => (
                group.as_deref().unwrap_or(""),
                name.as_deref().unwrap_or(""),
            ),
            Dependency::Project { .. } => return false,
        };
        [group, name]
            .iter()
            .any(|field| starts_junit_family(field))
    }

    /// The external fact name this dependency states (`group:artifact`,
    /// the python manifest-declared-name parity); project dependencies are
    /// first-party and state no external name.
    pub(crate) fn external_name(&self) -> Option<String> {
        match self {
            Dependency::Coordinate { group, artifact, .. } => {
                Some(format!("{group}:{artifact}"))
            }
            Dependency::Catalog { alias, group, name, .. } => match (group, name) {
                (Some(group), Some(name)) => Some(format!("{group}:{name}")),
                _ => Some(alias.clone()),
            },
            Dependency::Project { .. } => None,
        }
    }
}

/// The junit family predicate (K-P4): `junit*` or `org.junit*`.
fn starts_junit_family(field: &str) -> bool {
    let field = field.to_lowercase();
    field.starts_with("junit") || field.starts_with("org.junit")
}

/// The static facts of one build script: dependency rows plus the literal
/// build-directory relocation (K-P7).
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildFacts {
    pub(crate) dependencies: Vec<Dependency>,
    /// A literal `layout.buildDirectory.set(file("…"))` in THIS file: the
    /// relocation the driver's own exclusion set honors.
    pub(crate) build_dir_override: Option<String>,
}

pub(crate) fn scan_build_file(path: &Path) -> Result<BuildFacts, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(scan_build_source(&raw))
}

pub(crate) fn scan_build_source(raw: &str) -> BuildFacts {
    let mut facts = BuildFacts::default();
    // A `dependencies {` opener written on the same line as its calls (the
    // one-line dialect) gets the opener's block onto its own line: one
    // static rewrite keeps the depth walk below shape-independent. Calls
    // the walk cannot place inside the block state nothing (the static
    // visibility law), never a silent miss.
    let raw = raw.replace("dependencies {", "dependencies {\n");
    let mut inside_dependencies = false;
    let mut depth_at_block = 0usize;
    let mut depth = 0usize;
    for line in raw.lines() {
        let line = match line.find("//") {
            Some(cut) => &line[..cut],
            None => line,
        };
        let opened = line.matches('{').count();
        let closed = line.matches('}').count();
        if !inside_dependencies && line.trim() == "dependencies {" {
            inside_dependencies = true;
            depth_at_block = depth;
            depth += opened.saturating_sub(closed);
            continue;
        }
        if inside_dependencies {
            if let Some(dependency) = parse_dependency_line(line) {
                facts.dependencies.push(dependency);
            }
            depth += opened;
            depth = depth.saturating_sub(closed);
            if depth == depth_at_block && closed > 0 {
                inside_dependencies = false;
            }
        } else {
            depth += opened;
            depth = depth.saturating_sub(closed);
        }
        if facts.build_dir_override.is_none()
            && let Some(path) = build_directory_literal(line)
        {
            facts.build_dir_override = Some(path);
        }
    }
    facts.dependencies.sort();
    facts.dependencies.dedup();
    facts
}

/// One `configuration(args)` dependency line inside a dependencies block.
fn parse_dependency_line(line: &str) -> Option<Dependency> {
    let line = line.trim();
    let open = line.find('(')?;
    let configuration = line[..open].trim();
    if configuration.is_empty()
        || !DEPENDENCY_CONFIGURATIONS
            .iter()
            .any(|known| known == &configuration.to_lowercase())
    {
        return None;
    }
    let arguments = line[open + 1..].split(')').next().unwrap_or("").trim();
    // Only single-argument dependency calls are read; the multi-arg and
    // map forms state nothing here (the static visibility law).
    if arguments.contains(',') {
        return None;
    }
    if let Some(inner) = arguments.strip_prefix("project(") {
        let inner = inner.trim();
        let quoted = strip_quotes(inner)?;
        let path = quoted.strip_prefix(':')?;
        return Some(Dependency::Project {
            path: path.split(':').collect::<Vec<_>>().join("::"),
        });
    }
    if let Some(rest) = arguments.strip_prefix("libs.") {
        let alias = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
            .collect::<String>();
        return Some(Dependency::Catalog {
            alias,
            group: None,
            name: None,
            configuration: configuration.to_string(),
        });
    }
    let quoted = strip_quotes(arguments)?;
    let mut segments = quoted.split(':');
    let group = segments.next()?;
    let artifact = segments.next()?;
    if group.is_empty() || artifact.is_empty() {
        return None;
    }
    Some(Dependency::Coordinate {
        group: group.to_string(),
        artifact: artifact.to_string(),
        configuration: configuration.to_string(),
    })
}

fn strip_quotes(text: &str) -> Option<&str> {
    text.trim()
        .strip_prefix('"')
        .and_then(|tail| tail.strip_suffix('"'))
        .or_else(|| text.trim().strip_prefix('\'').and_then(|tail| tail.strip_suffix('\'')))
}

fn build_directory_literal(line: &str) -> Option<String> {
    if !line.contains("layout.buildDirectory.set(") {
        return None;
    }
    let open = line.find("file(")?;
    let arguments = line[open + "file(".len()..].split(')').next()?;
    let path = strip_quotes(arguments)?;
    Some(path.trim_end_matches('/').trim_start_matches("./").to_string())
}

/// The root version catalog: `gradle/libs.versions.toml` `[libraries]`.
/// `Err` states the K-P4 degrade class (the file is present but cannot be
/// parsed); `Ok(None)` is the plain absence — no alias evidence, no line.
pub(crate) fn catalog(root: &Path) -> Result<Option<Catalog>, String> {
    let path = root.join("gradle").join("libs.versions.toml");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    match raw.parse::<toml::Value>() {
        Ok(value) => Ok(Some(Catalog {
            libraries: libraries(&value),
        })),
        Err(_) => Err(
            "gradle catalog unreadable: gradle/libs.versions.toml — test-tier evidence unavailable"
                .to_string(),
        ),
    }
}

/// The alias → (group, name) map the accessors resolve through.
pub(crate) struct Catalog {
    libraries: Vec<(String, String, String)>,
}

fn libraries(value: &toml::Value) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    let Some(entries) = value.get("libraries").and_then(toml::Value::as_table) else {
        return rows;
    };
    for (alias, entry) in entries {
        match entry {
            toml::Value::String(coordinate) => {
                // The shorthand form `"group:name:version"` (the version
                // tail is never read).
                let mut segments = coordinate.split(':');
                if let (Some(group), Some(name)) = (segments.next(), segments.next())
                    && !group.is_empty()
                    && !name.is_empty()
                {
                    rows.push((alias.clone(), group.to_string(), name.to_string()));
                }
            }
            toml::Value::Table(table) => {
                let group = table.get("group").and_then(toml::Value::as_str);
                let name = table.get("name").and_then(toml::Value::as_str);
                // The one-string form `{ module = "group:name", … }`.
                let module = table
                    .get("module")
                    .and_then(toml::Value::as_str)
                    .map(str::to_string);
                match (group, name, module) {
                    (Some(group), Some(name), _) => {
                        rows.push((alias.clone(), group.to_string(), name.to_string()));
                    }
                    (_, _, Some(module)) => {
                        let mut segments = module.split(':');
                        if let (Some(group), Some(name)) = (segments.next(), segments.next()) {
                            rows.push((alias.clone(), group.to_string(), name.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    rows.sort();
    rows
}

impl Catalog {
    /// Resolve one accessor alias (dots from the TOML nesting, e.g. `junit`
    /// or `androidx.core`).
    pub(crate) fn resolve(&self, alias: &str) -> Option<(&str, &str)> {
        self.libraries
            .iter()
            .find(|(row_alias, _, _)| *row_alias == alias)
            .map(|(_, group, name)| (group.as_str(), name.as_str()))
    }
}

/// The external-name set a dependency list states (manifest parity; the
/// unit_manifests row content).
pub(crate) fn external_names(dependencies: &[Dependency]) -> BTreeSet<String> {
    dependencies
        .iter()
        .filter_map(Dependency::external_name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{scan_build_source, Dependency};

    #[test]
    fn inline_gav_and_project_and_alias_shapes_all_read() {
        let facts = scan_build_source(
            "dependencies {\n    implementation(\"androidx.core:core-ktx:1.12.0\")\n    testImplementation(\"org.junit.jupiter:junit-jupiter:5.10.2\")\n    implementation(project(\":core\"))\n    testImplementation(libs.junit)\n}\n",
        );
        assert_eq!(facts.dependencies.len(), 4, "{:?}", facts.dependencies);
        assert!(facts
            .dependencies
            .iter()
            .any(|dependency| matches!(dependency, Dependency::Project { path } if path == "core")));
        let junit = facts
            .dependencies
            .iter()
            .find(|dependency| matches!(dependency, Dependency::Coordinate { group, .. } if group == "org.junit.jupiter"))
            .expect("inline junit gav");
        assert!(junit.junit_evidence(), "inline GAV is evidence (K-P4)");
    }

    #[test]
    fn the_junit_family_predicate_is_group_or_name_not_the_bare_name() {
        // A dependency on a library NAMED junit in a NON-junit family group
        // is no evidence: family reads group OR name for `junit*`/`org.junit*`.
        let facts = scan_build_source(
            "dependencies {\n    testImplementation(\"com.acme:junit-toolkit:1.0\")\n}\n",
        );
        assert!(facts.dependencies[0].junit_evidence(), "name matches junit*");
        let facts = scan_build_source(
            "dependencies {\n    implementation(\"org.junit.jupiter:jupiter:5.0\")\n}\n",
        );
        assert!(
            !facts.dependencies[0].junit_evidence(),
            "the family alone is not the TEST configuration half"
        );
    }

    #[test]
    fn configuration_outside_dependencies_is_not_read() {
        let facts = scan_build_source("plugins {\n    id(\"junit\")\n}\n");
        assert!(facts.dependencies.is_empty());
    }

    #[test]
    fn project_deps_state_no_external_name() {
        let facts = scan_build_source("dependencies {\n    implementation(project(\":core\"))\n}\n");
        assert_eq!(super::external_names(&facts.dependencies), Default::default());
    }

    #[test]
    fn literal_build_directory_relocation_is_stated() {
        let facts = scan_build_source(
            "plugins { kotlin(\"jvm\") }\nlayout.buildDirectory.set(file(\"out\"))\n",
        );
        assert_eq!(facts.build_dir_override.as_deref(), Some("out"));
    }
}
