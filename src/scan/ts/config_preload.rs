//! The tsconfig pre-read — the delegation read set validated BEFORE any
//! resolution runs (ADR-024, resolution record R7c: the pre-read law,
//! workplan_archspec_ts_resolution_completeness round-2 F3).
//!
//! archspec reads every tsconfig the run will delegate against before the
//! first resolution call, and a pre-read file that EXISTS but cannot be
//! read or parsed reddens the run as `tsconfig load failed: <path>` (R7c).
//! The pinned crate cannot carry this verdict alone: its Auto discovery
//! skips an unreadable candidate mid-walk and continues (measured source:
//! "Skip unreadable tsconfig files… and continue walking"), so an alias-free
//! green could otherwise hide a config the run never read — the
//! silent-blindness class R7c outlaws. This module owns the pre-read only:
//! existence, readability and parseability over the closure, never a second
//! tsconfig dialect and never an alias interpretation (R7c, round-2 F3).
//!
//! The pre-read set is BOUNDED like discovery (R2, round-2 N4): it starts at
//! the owning config the caller computed with [`super::config_chain::owning_tsconfig`]
//! and walks the `extends` closure transitively — own config plus every file
//! the closure names, at ANY file name (the fixture's own chain extends a
//! `tsconfig.base.json`, which a `tsconfig.json`-only rule misses). Where no
//! owning config exists within the bound the file lives in the inferred
//! project: the preload is empty and honest, never a fault (R2 absence law).
//! Files no scanned file owns and no read chain names are never read — a
//! broken config in a never-referenced directory stays green.
//!
//! Extends TARGETS are named the way the pinned crate names them, verified
//! against the pinned source rather than guessed: a `.`-prefixed specifier
//! joins the declaring config's directory, an absolute one is taken verbatim,
//! and a bare or `#`-prefixed specifier goes through the crate's own node
//! resolution — bare-specifier `extends` DOES resolve through `node_modules`
//! at this pin (`get_extended_tsconfig_path` hands such specifiers to its
//! `tsconfig_extends_resolver`, conditions `node`+`import`, extensions
//! `.json`, mainFiles `tsconfig`). Candidate spelling follows the crate's
//! loader: an existing file as-is, a directory gains `tsconfig.json`,
//! anything else gains a `.json` suffix. Closure members dedupe by CANONICAL
//! path; an `extends` chain that loops back onto a config still on the walk
//! terminates and reports `tsconfig cycle (extends or reference)` (R7e) —
//! its own class, never a load failure (round-1 F2: the cycle IS the fault).
//!
//! Part B of this lane turns [`TsconfigPreloadError`] into the R7 finding
//! strings here: the resolution seam (`resolution.rs`) calls
//! [`preload_tsconfigs`] ahead of every delegation and carries each fault
//! into the findings channel through its Display, the structured fields
//! riding along for the record's exact-message law (round-2 F6: the
//! as-declared specifier beside the attempted path).

use oxc_resolver::{ExtendsField, PathUtil, ResolveError, ResolveOptions, Resolver, TsConfig};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::config_chain::TSCONFIG_FILE;

/// One pre-read closure member: the config FILE as parsed (canonical
/// spelling — the dedupe identity, R7c) and its parsed content. The seam
/// hands `path` to the delegation entry and formats faults from `path`;
/// the parsed content feeds the alias map (message-only candidates, R7a) —
/// the delegation itself re-parses through the crate, so no merge semantics
/// live in this module.
#[derive(Debug)]
pub(crate) struct PreloadedTsconfig {
    pub(crate) path: PathBuf,
    pub(crate) tsconfig: TsConfig,
}

/// The pre-read fault classes, one per R7 tsconfig class part B formats.
/// The Display lines match the emitted-finding strings the record defines;
/// the fields beside them carry what the R7 exact-message formatting needs.
#[derive(Debug)]
#[allow(dead_code, reason = "the R7 exact-message payload fields ride per round-2 F6 — Display spells the finding bytes and the seam formats through it; the fields stay structured for the record's formatting law")]
pub(crate) enum TsconfigPreloadError {
    /// A pre-read file EXISTS but could not be read or parsed — R7c class
    /// (c), part B formats `tsconfig load failed: <path>` (exit 1). `source`
    /// is the reader's or parser's own message for the detail line.
    LoadFailed { path: PathBuf, source: String },
    /// An `extends` target is missing — R7c class (c), part B formats
    /// `tsconfig not found: <target the resolver attempted>` (exit 1).
    /// `specifier` is the extends string VERBATIM as declared (round-2 F6);
    /// `attempted` mirrors the crate's `TsconfigNotFound` payload at this
    /// pin: the joined path for a relative extends, the specifier itself for
    /// a bare or `#` one (both measured, R7c). `tsconfig` names the config
    /// that declared the extends.
    ExtendsNotFound {
        tsconfig: PathBuf,
        specifier: String,
        attempted: PathBuf,
    },
    /// An extends chain loops — R7 class (e), part B formats
    /// `tsconfig cycle (extends or reference): <paths>` (exit 1). `path` is
    /// the repeated config; `chain` spells the walk that closed the loop,
    /// mirroring the crate's `TsconfigCircularExtend` payload shape. The
    /// detection is the pre-read's own (canonical-path stack): a cycle can
    /// never reach the delegation, and the walk terminates by construction.
    ExtendsCycle { path: PathBuf, chain: Vec<PathBuf> },
}

impl fmt::Display for TsconfigPreloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed { path, .. } => write!(f, "tsconfig load failed: {}", path.display()),
            Self::ExtendsNotFound { attempted, .. } => {
                write!(f, "tsconfig not found: {}", attempted.display())
            }
            Self::ExtendsCycle { chain, .. } => {
                write!(f, "tsconfig cycle (extends or reference):")?;
                for (index, path) in chain.iter().enumerate() {
                    if index != 0 {
                        write!(f, " ->")?;
                    }
                    write!(f, " {}", path.display())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for TsconfigPreloadError {}

/// Validate and collect the delegation read set of one owning tsconfig
/// BEFORE any resolution happens (the F3 pre-read law, R7c).
///
/// `entry` is the owning config the caller computed with
/// [`super::config_chain::owning_tsconfig`]. `None` is the inferred project
/// — no config inside the discovery bound owns the importing files — and
/// yields `Ok(vec![])`: absence is not a fault class (R2), so the preload
/// states no fact and reddens nothing. Every config in the extends closure
/// is read and parsed before the function returns, deduped by canonical
/// path; a broken member fails the whole preload on its first fault.
pub(crate) fn preload_tsconfigs(
    entry: Option<&Path>,
) -> Result<Vec<PreloadedTsconfig>, TsconfigPreloadError> {
    let Some(entry) = entry else {
        // The inferred project (R2 absence law): empty preload, honest green.
        return Ok(Vec::new());
    };
    let extends = extends_resolver();
    let mut preloaded = Vec::new();
    let mut visited: BTreeSet<PathBuf> = BTreeSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    visit(entry, entry, None, &extends, &mut visited, &mut stack, &mut preloaded)?;
    Ok(preloaded)
}

/// One closure member: validate readability and parseability, then recurse
/// into its declared extends. `target` is the path this walk attempts;
/// `attempted` is the spelling R7c's not-found line names (the crate's
/// `TsconfigNotFound` payload shape: joined path or specifier itself);
/// `declaration` names the declaring config and as-declared specifier for an
/// extends-reached member, `None` for the entry.
fn visit(
    target: &Path,
    attempted: &Path,
    declaration: Option<(&Path, &str)>,
    extends: &Resolver,
    visited: &mut BTreeSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    preloaded: &mut Vec<PreloadedTsconfig>,
) -> Result<(), TsconfigPreloadError> {
    // Candidate spelling per the crate's loader at this pin
    // (`get_tsconfig`): existing file as-is, directory gains the owning
    // config name, anything else gains a `.json` suffix. fs::metadata
    // follows symlinks, matching the loader's classification for the
    // link-to-file case (the bytes read are identical either way).
    let candidate = candidate_file(target);
    let text = match fs::read_to_string(&candidate) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(match declaration {
                // F6: the missing target is reported with the as-declared
                // specifier; the attempted path travels beside it so part B
                // can name the target the resolver attempted.
                Some((config, specifier)) => TsconfigPreloadError::ExtendsNotFound {
                    tsconfig: config.to_path_buf(),
                    specifier: specifier.to_string(),
                    attempted: attempted.to_path_buf(),
                },
                // The entry vanishing between owning_tsconfig and here is a
                // race, not an extends miss — it names itself as a load
                // failure (no extends declaration exists to blame).
                None => TsconfigPreloadError::LoadFailed {
                    path: candidate.clone(),
                    source: err.to_string(),
                },
            });
        }
        // EXISTS but unreadable (the R7c defect variant: permission denied).
        // The crate's Auto walk would skip it and keep walking — the pre-read
        // reddens so no alias-free green hides an ignored config.
        Err(err) => {
            return Err(TsconfigPreloadError::LoadFailed {
                path: candidate.clone(),
                source: err.to_string(),
            });
        }
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|err| TsconfigPreloadError::LoadFailed {
            path: candidate.clone(),
            source: err.to_string(),
        })?;
    if let Some(index) = stack.iter().position(|on_stack| *on_stack == canonical) {
        // R7e: the extends chain loops back onto a config still on the walk.
        // Reported as its own class, never a load failure, and the walk
        // terminates here — no recursion past a repeated member.
        let mut chain = stack[index..].to_vec();
        chain.push(canonical.clone());
        return Err(TsconfigPreloadError::ExtendsCycle { path: canonical, chain });
    }
    if visited.contains(&canonical) {
        // Diamond extends (one base reached twice): a single member.
        return Ok(());
    }
    // The pinned crate's own parser — JSONC with BOM and comment stripping,
    // one parse truth, never a second dialect. The root flag follows the
    // crate: the entry is the caller config, extends members are not.
    let parsed = TsConfig::parse(
        /* root */ declaration.is_none(),
        &candidate,
        &canonical,
        text,
    )
    .map_err(|err| TsconfigPreloadError::LoadFailed {
        path: candidate.clone(),
        source: err.to_string(),
    })?;
    let specifiers: Vec<String> = match &parsed.extends {
        // Same reading as the crate's own extends() at this pin: a single
        // string or an array, walked in DECLARED order (the closure set is
        // order-independent; the crate's reverse iteration is its own
        // merge-precedence concern, which lives in the delegation, here).
        Some(ExtendsField::Single(specifier)) => vec![specifier.clone()],
        Some(ExtendsField::Multiple(specifiers)) => specifiers.clone(),
        None => Vec::new(),
    };
    visited.insert(canonical.clone());
    preloaded.push(PreloadedTsconfig {
        path: canonical.clone(),
        tsconfig: parsed,
    });
    // Joins anchor at the candidate directory, the crate's own anchor for
    // relative extends (`get_extended_tsconfig_path` uses the config's
    // directory as loaded); canonical paths stay the identity sets above.
    let directory = candidate.parent().expect("a config candidate names a file");
    stack.push(canonical);
    for specifier in &specifiers {
        let (target, attempted) = extend_target(&candidate, directory, specifier, extends)?;
        visit(
            &target,
            &attempted,
            Some((&candidate, specifier)),
            extends,
            visited,
            stack,
            preloaded,
        )?;
    }
    stack.pop();
    Ok(())
}

/// The extends-target path, spelled per the crate's
/// `get_extended_tsconfig_path` at this pin: empty is a specifier fault,
/// `/` is verbatim, `.` joins the config directory, anything else (bare or
/// `#`) goes through node resolution. The returned `attempted` mirrors the
/// crate's `TsconfigNotFound` payload: the joined/verbatim path for path
/// forms, the specifier itself for node-resolved forms.
fn extend_target(
    declaring: &Path,
    directory: &Path,
    specifier: &str,
    extends: &Resolver,
) -> Result<(PathBuf, PathBuf), TsconfigPreloadError> {
    match specifier.as_bytes().first() {
        None => Err(TsconfigPreloadError::LoadFailed {
            // The crate answers `Specifier(Empty)` here; part B's exhaustive
            // seam re-maps that variant. The pre-read names the config whose
            // extends cannot even be interpreted.
            path: declaring.to_path_buf(),
            source: String::from("extends specifier is empty"),
        }),
        Some(b'/') => Ok((PathBuf::from(specifier), PathBuf::from(specifier))),
        Some(b'.') => {
            let joined = directory.normalize_with(specifier);
            Ok((joined.clone(), joined))
        }
        // Bare and `#` specifiers resolve through node_modules / the nearest
        // imports map — verified behavior of the pinned crate, not a path
        // guess. NotFound answers the not-found class naming the specifier;
        // PackageImportNotDefined rides the same class (the crate's `#` arm
        // maps it there; a bare specifier cannot produce it); every other
        // crate error is carried as a load failure for part B's exhaustive
        // seam to re-map — never silence.
        _ => match extends.resolve(directory, specifier) {
            Ok(resolution) => Ok((resolution.into_path_buf(), PathBuf::from(specifier))),
            Err(ResolveError::NotFound(_) | ResolveError::PackageImportNotDefined(..)) => {
                Err(TsconfigPreloadError::ExtendsNotFound {
                    tsconfig: declaring.to_path_buf(),
                    specifier: specifier.to_string(),
                    attempted: PathBuf::from(specifier),
                })
            }
            Err(err) => Err(TsconfigPreloadError::LoadFailed {
                path: declaring.to_path_buf(),
                source: err.to_string(),
            }),
        },
    }
}

/// The mirror of the crate's own extends resolver at this pin
/// (`tsconfig_extends_resolver`): package resolution for bare and `#`
/// extends specifiers — conditions `node`+`import`, extensions `.json`,
/// mainFiles `tsconfig`, no tsconfig discovery of its own. Delegation to the
/// same crate with the same option shape keeps the crate's semantics; a
/// hand-rolled node_modules walk would be the second dialect R7c forbids.
fn extends_resolver() -> Resolver {
    Resolver::new(ResolveOptions {
        tsconfig: None,
        condition_names: vec!["node".to_string(), "import".to_string()],
        extensions: vec![".json".to_string()],
        main_files: vec!["tsconfig".to_string()],
        ..ResolveOptions::default()
    })
}

/// The candidate spelling law of the crate's tsconfig loader: an existing
/// file as-is, a directory gains `tsconfig.json`, anything else gains a
/// `.json` suffix (an extends target `./base` reads `base.json`).
fn candidate_file(path: &Path) -> PathBuf {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => path.to_path_buf(),
        Ok(meta) if meta.is_dir() => path.join(TSCONFIG_FILE),
        _ => with_json_suffix(path),
    }
}

fn with_json_suffix(path: &Path) -> PathBuf {
    let mut spelled = path.as_os_str().to_os_string();
    spelled.push(".json");
    PathBuf::from(spelled)
}

#[cfg(test)]
mod tests {
    use super::{TsconfigPreloadError, preload_tsconfigs};
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    struct Tree(tempfile::TempDir);

    impl Tree {
        fn new() -> Self {
            Self(tempfile::tempdir().expect("tempdir"))
        }
        fn root(&self) -> PathBuf {
            self.0.path().to_path_buf()
        }
        fn write(&self, rel: &str, content: &str) -> PathBuf {
            let path = self.root().join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(&path, content).expect("write");
            path
        }
        fn canonical(&self, rel: &str) -> PathBuf {
            self.root().join(rel).canonicalize().expect("canonical")
        }
        fn member_paths(tree: &Tree, rels: &[&str]) -> BTreeSet<PathBuf> {
            rels.iter().map(|rel| tree.canonical(rel)).collect()
        }
    }

    fn preloaded_paths(preloaded: &[super::PreloadedTsconfig]) -> BTreeSet<PathBuf> {
        preloaded.iter().map(|member| member.path.clone()).collect()
    }

    #[test]
    fn extends_chain_and_array_extends_preload_the_full_closure() {
        // Green: entry a → [b, style] with b → grand; every member of the
        // transitive closure — string or array extends — preloads Ok. The
        // comment line proves the crate's own JSONC parser reads the file
        // (no second dialect strips comments here).
        let tree = Tree::new();
        let entry = tree.write(
            "tsconfig.json",
            "// base chain\n{\"extends\": [\"./base.json\", \"./style.json\"]}",
        );
        tree.write("base.json", "{\"extends\": \"./grand.json\"}");
        tree.write("grand.json", "{}");
        tree.write("style.json", "{\"compilerOptions\": {\"strict\": true}}");
        let preloaded = preload_tsconfigs(Some(&entry)).expect("green closure preloads");
        assert_eq!(
            preloaded_paths(&preloaded),
            Tree::member_paths(&tree, &["tsconfig.json", "base.json", "grand.json", "style.json"])
        );
    }

    #[test]
    fn missing_extends_target_names_the_declared_specifier() {
        // Red (R7c class (c), F6): the extends target is absent — the error
        // carries the as-declared specifier beside the attempted path.
        let tree = Tree::new();
        let entry = tree.write("tsconfig.json", "{\"extends\": \"./missing.json\"}");
        let error = preload_tsconfigs(Some(&entry)).expect_err("missing extends is red");
        let TsconfigPreloadError::ExtendsNotFound {
            tsconfig,
            specifier,
            attempted,
        } = &error
        else {
            panic!("expected ExtendsNotFound, got {error:?}");
        };
        assert_eq!(tsconfig, &entry);
        assert_eq!(specifier, "./missing.json");
        assert_eq!(attempted, &tree.root().join("missing.json"));
        assert_eq!(
            error.to_string(),
            format!(
                "tsconfig not found: {}",
                tree.root().join("missing.json").display()
            )
        );
    }

    #[test]
    fn missing_indirect_extends_target_is_detected() {
        // Red: a → b → gone. Parsing a alone cannot see the fault — the
        // closure walk names the DECLARING config (b) and its specifier.
        let tree = Tree::new();
        let entry = tree.write("tsconfig.json", "{\"extends\": \"./base.json\"}");
        let base = tree.write("base.json", "{\"extends\": \"./gone.json\"}");
        let error = preload_tsconfigs(Some(&entry)).expect_err("indirect miss is red");
        let TsconfigPreloadError::ExtendsNotFound {
            tsconfig, specifier, ..
        } = &error
        else {
            panic!("expected ExtendsNotFound, got {error:?}");
        };
        assert_eq!(tsconfig, &base);
        assert_eq!(specifier, "./gone.json");
    }

    #[test]
    fn extends_cycle_terminates_as_its_own_class() {
        // Red (R7e): pkg → shared → pkg. The walk must terminate and report
        // the cycle — never a load failure (the cycle IS the fault) and
        // never an unbounded recursion.
        let tree = Tree::new();
        let entry = tree.write("pkg/tsconfig.json", "{\"extends\": \"../shared/base.json\"}");
        tree.write("shared/base.json", "{\"extends\": \"../pkg/tsconfig.json\"}");
        let error = preload_tsconfigs(Some(&entry)).expect_err("extends cycle is red");
        let TsconfigPreloadError::ExtendsCycle { path, chain } = &error else {
            panic!("expected ExtendsCycle, got {error:?}");
        };
        assert_eq!(path, &tree.canonical("pkg/tsconfig.json"));
        assert_eq!(
            chain,
            &[
                tree.canonical("pkg/tsconfig.json"),
                tree.canonical("shared/base.json"),
                tree.canonical("pkg/tsconfig.json"),
            ]
        );
        assert!(error.to_string().starts_with("tsconfig cycle (extends or reference):"));
    }

    #[test]
    fn truncated_tsconfig_is_load_failed_naming_the_path() {
        // Red (R7c class (c)): an existing config the parser rejects —
        // `tsconfig load failed: <path>`, the record's string verbatim.
        let tree = Tree::new();
        let entry = tree.write("tsconfig.json", "{\"extends\": \"./base.json\", ");
        let error = preload_tsconfigs(Some(&entry)).expect_err("unparseable is red");
        let TsconfigPreloadError::LoadFailed { path, source } = &error else {
            panic!("expected LoadFailed, got {error:?}");
        };
        assert_eq!(path, &entry);
        assert!(!source.is_empty(), "the parser message rides along");
        assert_eq!(
            error.to_string(),
            format!("tsconfig load failed: {}", entry.display())
        );
    }

    #[test]
    fn plain_config_preloads_one_member_and_no_config_preloads_nothing() {
        // Green: a config with no extends is a one-member closure; the
        // inferred project (no owning config, R2 absence law) is an empty
        // preload, never a fault.
        let tree = Tree::new();
        let entry = tree.write("tsconfig.json", "{\"compilerOptions\": {\"strict\": true}}");
        let preloaded = preload_tsconfigs(Some(&entry)).expect("plain config preloads");
        assert_eq!(preloaded.len(), 1);
        assert_eq!(preloaded[0].path, tree.canonical("tsconfig.json"));
        assert!(preload_tsconfigs(None).expect("inferred project preloads nothing").is_empty());
    }

    #[test]
    fn diamond_extends_preloads_the_shared_base_once() {
        // Dedupe by canonical path (R7c closure walk): a base reached twice
        // is ONE member — the walk stays terminating on diamonds too.
        let tree = Tree::new();
        let entry = tree.write(
            "tsconfig.json",
            "{\"extends\": [\"./left.json\", \"./right.json\"]}",
        );
        tree.write("left.json", "{\"extends\": \"./shared/base.json\"}");
        tree.write("right.json", "{\"extends\": \"./shared/../shared/base.json\"}");
        tree.write("shared/base.json", "{}");
        let preloaded = preload_tsconfigs(Some(&entry)).expect("diamond preloads green");
        assert_eq!(
            preloaded_paths(&preloaded),
            Tree::member_paths(&tree, &["tsconfig.json", "left.json", "right.json", "shared/base.json"])
        );
    }
}
