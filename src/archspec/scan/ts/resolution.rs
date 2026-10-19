//! The resolution seam (ADR-024, resolution record R1): the one choke-point
//! through which every bare and `#` specifier of the ts driver passes.
//!
//! The seam wraps the pinned `oxc_resolver`: one `Resolver` cached per owning
//! config (the bounded chain archspec computes itself — the crate's `Auto`
//! discovery is never used, R2), pre-read through `config_preload` ahead of
//! every delegation (R7c), and answered by the exhaustive `ResolveError`
//! taxonomy of the plan: every variant has exactly one home, a miss either
//! names itself as a finding (exit 1) or states the honest external fact.
//! tsconfig load/cycle/not-found faults, invalid package manifests and
//! class-less resolver errors are bare category findings; dead alias targets,
//! `#imports` misses and export-map misses ride the anchored
//! `unresolvable import` line; a declared-but-absent package adds the
//! report-only `external not installed` line and never reddens the run (R7b).
//! A BARE specifier naming a workspace member with no on-disk resolution
//! is a first-party membership edge with the report-only `undeclared
//! member import` line when the registry root's lockfile names a hoisting
//! family — under `pnpm-lock.yaml` or no lockfile the same miss keeps the
//! NotFound red (R5 case (iv), round-4 linker-family gate).

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::path::{Component, Path, PathBuf};

use oxc_resolver::{
    JSONError, ResolveError, ResolveOptions, Resolver, TsConfig, TsconfigDiscovery,
    TsconfigOptions, TsconfigReferences,
};

use super::config_chain::owning_tsconfig;
use super::config_preload::{preload_tsconfigs, TsconfigPreloadError};
use super::gitignore::{self, IgnoreHit};
use crate::archspec::language;
use super::common::{
    exclusion_hit, is_excluded, is_virtual_spec, package_identity, relative, strip_query,
    Exclusions, Resolved,
};

/// The canonical Node built-in names (resolution record N3 builtins law): a
/// specifier whose first path segment matches stays an external fact —
/// never delegated, never unresolvable (the npm package named `path`
/// collides onto the same external fact by law).
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "assert/strict",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "stream/promises",
    "string_decoder",
    "sys",
    "test_runner",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
];

/// True when the specifier's first path segment is a Node built-in. Scoped
/// names never match: `@scope/path` is a package, not `path`.
pub(crate) fn is_builtin_spec(specifier: &str) -> bool {
    let name = specifier.strip_prefix("node:").unwrap_or(specifier);
    if name.starts_with('@') {
        return false;
    }
    let head = name.split('/').next().unwrap_or(name);
    NODE_BUILTINS.contains(&head)
}

/// The condition set the delegation runs with (R3): WITHOUT `types` —
/// types-only export maps ride the class-(f) exports miss instead. The
/// resolver adds the `default` condition implicitly.
const CONDITION_NAMES: &[&str] = &["node", "import", "require", "svelte"];

/// The extension list the pinned plan commits to.
const RESOLVE_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".svelte", ".json"];

/// One cached entry per owning config (`Option` key: `None` is the inferred
/// project — alias-free, never a fault, R2 absence law).
enum Entry {
    /// The delegation engine plus the preloaded closure configs (entry
    /// first, then closure order) that feed the message-only alias map.
    Ready {
        resolver: Box<Resolver>,
        configs: Vec<TsConfig>,
    },
    /// The pre-read faulted; the fault line was pushed into findings once
    /// and every later bare miss on this entry states the external fact
    /// without delegating (the finding names it; exit 1 carries the verdict).
    Faulted,
}

/// The resolution seam of one ts extraction.
pub(crate) struct Engine {
    /// Canonicalized scan root: the under-root test and the repo-relative
    /// spellings both key on it.
    root: PathBuf,
    /// The spec's declared exclusions (the named exclusion an excluded
    /// resolution hit prints).
    declared: Option<Exclusions>,
    /// Engine per owning config, built on first sight.
    entries: HashMap<Option<PathBuf>, Entry>,
    /// Bare-category findings, generated order; the join-and-Err in extract
    /// turns them into the exit-1 verdict.
    findings: Vec<String>,
    /// Dedup identities for findings (faults by message, invalid manifests
    /// by path).
    seen_faults: BTreeSet<String>,
    seen_manifests: BTreeSet<PathBuf>,
    /// The `external not installed` lines (R7b): dedup, never red, drained
    /// into the model's transient report field.
    report_only: BTreeSet<String>,
    /// Guard G cache per (dir, package identity): the first existing but
    /// unreadable-or-unparseable package.json the walk found, if any.
    guard_cache: HashMap<(PathBuf, String), Option<PathBuf>>,
    /// Declared-name sets per manifest path (the class-(b) declared test).
    declared_cache: HashMap<PathBuf, BTreeSet<String>>,
    /// The workspace registry (R5): empty on every single-package tree —
    /// the precedence law engages only when the registry proves members.
    registries: Vec<super::workspace::Registry>,
    /// Parsed `.gitignore` cache (one parse per file per scan).
    gitignore: gitignore::Evaluator,
    /// Generated-source verdicts per probed base (perf lane 2026-10-17,
    /// fix 2): the candidate loops re-probe the same absent bases (the
    /// `.or_else` spelling pair, sibling candidate paths). The verdict of
    /// a base is a function of the path alone over the committed tree,
    /// which cannot change during a scan — one walk per base.
    gensrc: RefCell<HashMap<PathBuf, Option<IgnoreHit>>>,
    /// Member manifest memo per member dir (perf lane 2026-10-17, fix 3).
    member_manifests: RefCell<HashMap<PathBuf, Option<Arc<serde_json::Value>>>>,
    /// Bare-resolution memo per (importing dir, spelled specifier) (perf
    /// lane 2026-10-17, fix 4): the edge loop re-enters `classify_bare`
    /// for repeated (file-parent, specifier) pairs (measured 819/43652
    /// repeats on open-mercato). The verdict is a function of the pair
    /// over the committed tree, and every side-effect channel the body
    /// writes (`report_only`, `seen_faults`, `seen_manifests`, `entries`)
    /// is a deduped set — a hit replays the identical verdict and owes
    /// nothing further.
    bare: RefCell<HashMap<(PathBuf, String), Resolved>>,
    /// The `manifest_unreadable` verdict per FULL PROBE PATH (round 9,
    /// P-PAR3): guard_g's upward walk re-probes the same manifest paths
    /// across (dir, package) keys (measured stat storm: 253,226 `stat`s
    /// on open-mercato, 57 % guard_cache hits — the walk-answer cache
    /// above stays, this memo caches the PER-PATH PREDICATE). The law is
    /// the `gensrc` memo's: the verdict is a function of the path alone
    /// over the committed tree, which cannot change during a scan. A
    /// TRUE verdict memoizes under the `guard_cache` precedent — that
    /// cache already stores, per scan, the walk that NAMED an
    /// existing-but-unreadable-or-unparseable manifest. The tension with
    /// `member_manifest`'s deliberate non-memo of parse failures
    /// ("a parse failure is not a stable fact — the file exists — a
    /// later read could hit a different parse of changed bytes on
    /// mutable trees") is NAMED, not hidden: this memo accepts the same
    /// staleness admission `guard_cache` already made. The key is the
    /// full probe path, NEVER the directory: guard_g's second probe
    /// embeds the package name, so two packages probed from one dir ask
    /// about DIFFERENT files (round 9, F15).
    manifest_verdicts: RefCell<HashMap<PathBuf, bool>>,
    /// True when the spec at the scan root declares `framework = "sveltekit"`
    /// for the ts driver (round-8 P-FW1: the key selects the framework
    /// resolution semantics — L-C). No filesystem sniffing participates:
    /// the `svelte.config.js` presence probe is DELETED (a file's existence
    /// is tree-probing, the discovery class the operator ruling closes).
    sveltekit: bool,
}

impl Engine {
    /// The seam for one extraction run rooted at `root` with the spec's
    /// declared exclusions and the declared framework key (round-8: the
    /// driver reads the `[project] framework` value through this seam —
    /// validation already happened at spec load, one owner).
    pub(crate) fn new(
        root: &Path,
        declared: Option<&Exclusions>,
        framework: Option<&str>,
    ) -> Self {
        Self {
            root: root.to_path_buf(),
            declared: declared.map(|exclusions| Exclusions {
                declared: exclusions.declared.clone(),
            }),
            entries: HashMap::new(),
            findings: Vec::new(),
            seen_faults: BTreeSet::new(),
            seen_manifests: BTreeSet::new(),
            report_only: BTreeSet::new(),
            guard_cache: HashMap::new(),
            declared_cache: HashMap::new(),
            registries: Vec::new(),
            gitignore: gitignore::Evaluator::new(root),
            gensrc: RefCell::new(HashMap::new()),
            member_manifests: RefCell::new(HashMap::new()),
            bare: RefCell::new(HashMap::new()),
            manifest_verdicts: RefCell::new(HashMap::new()),
            sveltekit: framework == Some("sveltekit"),
        }
    }

    /// Attach the workspace registry (R5): the driver calls this once per
    /// extract when the tree carries a workspace root.
    pub(crate) fn set_workspace(&mut self, registries: Vec<super::workspace::Registry>) {
        self.registries = registries;
    }

    /// The registry dir owning a member NAME (nearest workspace root
    /// first); `None` for every name the registry does not declare.
    fn workspace_member_dir(&self, name: &str) -> Option<PathBuf> {
        self.workspace_member_hit(name)
            .map(|(member_dir, _root)| member_dir)
    }

    /// The member hit for a NAME plus the ROOT of the registry declaring
    /// it: the FIRST registered member for that name, nearest scan root
    /// first — the registry is deterministic, not claimed unique (R5 round
    /// 4). The root carries the linker-family signal of case (iv).
    fn workspace_member_hit(&self, name: &str) -> Option<(PathBuf, PathBuf)> {
        for registry in &self.registries {
            for member in &registry.members {
                if member.name == name {
                    return Some((member.dir.clone(), registry.root.clone()));
                }
            }
        }
        None
    }

    /// R5 case (ii): the resolved path landed outside the member dir while
    /// the importing manifest DECLARES the member (round-2 N5) — the
    /// external fact stands and one report-only line is owed. Case (i) —
    /// the resolution landed inside the member dir — owes no line.
    fn workspace_link_missing(&mut self, dir: &Path, identity: &str, resolved: &Path) {
        let Some(member_dir) = self.workspace_member_dir(identity) else {
            return;
        };
        if resolved.starts_with(&member_dir) {
            return;
        }
        if self.declared_nearest_manifest(dir, identity).is_none() {
            return;
        }
        let spelled = relative(resolved, &self.root)
            .unwrap_or_else(|| resolved.to_string_lossy().replace('\\', "/"));
        self.report_only.insert(format!(
            "workspace link missing: {identity} — resolved to {spelled} instead of the member dir"
        ));
    }

    /// The context-free member law (R5 as replaced round-6 P-CTX) plus
    /// source-always-for-members (round-6 P-DIST) plus the generated-source
    /// class (round-7 P-GENSRC): a member-named specifier that resolved
    /// nowhere is answered archspec-side by the TARGET member's own
    /// declarations — MEMBERSHIP adjudicates first-party, the importer's
    /// context never gates it. Declaration state selects the report-only
    /// attribution only: the declared arm keeps the case-(iii) `workspace
    /// link missing` line and the entry fallback; the undeclared arm keeps
    /// the round-4 hoisting-family gate for the BARE no-answer case; a
    /// member subpath resolves under EVERY family. Where nothing lands and
    /// the absent target is gitignored, the R12 line rides instead of the
    /// red (fail closed: no `.gitignore` files keep the red).
    fn classify_member(
        &mut self,
        member_dir: &Path,
        registry_root: &Path,
        importing: &Path,
        specifier: &str,
        identity: &str,
        declared_in: Option<PathBuf>,
    ) -> Resolved {
        let subpath = specifier
            .strip_prefix(identity)
            .map(|rest| rest.trim_matches('/'))
            .unwrap_or("");
        if !subpath.is_empty() {
            // MEMBER-NAMED SUBPATH (round-6 P-CTX): the derivation order
            // (record R6) is the TARGET member's exports map under the R3
            // condition set, THEN the member dir joined with the spelled
            // remainder plus the extension/index table — under every
            // linker family, with no importer context consulted. The SAME
            // single key match hands the generated-attribution probe the
            // winning entry VALUE (plan D3) — one lookup, never a second
            // key-match (decision-check M2).
            let matched = self
                .member_manifest(member_dir)
                .and_then(|manifest| exports_subpath_target(&manifest, subpath));
            let mapped = matched
                .as_ref()
                .map(|(spelled, ..)| member_dir.join(spelled.trim_start_matches("./")));
            let base = member_dir.join(subpath);
            let found = mapped
                .as_deref()
                .and_then(member_candidates)
                .or_else(|| member_candidates(&base));
            let edge = found.as_deref().and_then(|f| self.disk_edge(f));
            if let Some(edge) = edge {
                if declared_in.is_some() {
                    self.report_only.insert(format!(
                        "workspace link missing: {identity} — absent from disk"
                    ));
                }
                return edge;
            }
            let generated = self
                .gensrc_absent(mapped.as_deref().unwrap_or(&base))
                .map(|hit| (hit, mapped.as_deref().unwrap_or(&base)))
                .or_else(|| self.gensrc_absent(&base).map(|hit| (hit, base.as_path())));
            if let Some((hit, declared)) = generated {
                // GENERATED ATTRIBUTION (plan D1): inside the generated
                // class, and ONLY where nothing exists (`found.is_none()`
                // — a built tree can never be re-routed, N2), the winning
                // entry's `types` arm may declare this subpath's SOURCE.
                // Exactly one existing candidate REPLACES the line with an
                // edge plus the existing `workspace target not built`
                // disclosure — its first slot spells the member-qualified
                // specifier (D2). Zero or many candidates keep the R12
                // line VERBATIM: the probe never resolves the import, so
                // TS's declaration-order precedence grants no authority to
                // position-pick (decision-check B2 law).
                if found.is_none()
                    && let Some((_, entry, capture)) = matched.as_ref()
                    && let Some(source) =
                        self.attributed_source(member_dir, entry, capture.as_deref())
                {
                    let spelled = relative(declared, &self.root)
                        .unwrap_or_else(|| declared.to_string_lossy().replace('\\', "/"));
                    self.report_only
                        .insert(format!("workspace target not built: {specifier} -> {spelled}"));
                    return self.disk_edge(&source).unwrap_or(Resolved::Reported);
                }
                return self.reported_generated(&hit, specifier);
            }
            return Resolved::Unresolvable(String::new());
        }
        // BARE member name: SOURCE-ALWAYS-FOR-MEMBERS (P-DIST) — the TS
        // derivation table, never a dist/build target. A declared target
        // that is ABSENT rides the report-only line, the edge lands on
        // the SOURCE answer.
        if let Some((source, not_built)) = self.member_source(member_dir) {
            if let Some(target) = not_built {
                let spelled = relative(&target, &self.root)
                    .unwrap_or_else(|| target.to_string_lossy().replace('\\', "/"));
                self.report_only
                    .insert(format!("workspace target not built: {identity} -> {spelled}"));
            }
            if declared_in.is_some() {
                self.report_only.insert(format!(
                    "workspace link missing: {identity} — absent from disk"
                ));
            } else if declared_in.is_none()
                && super::workspace::root_lockfile_hoists(registry_root)
                && specifier == identity
            {
                let spelled = relative(importing, &self.root)
                    .unwrap_or_else(|| importing.to_string_lossy().replace('\\', "/"));
                self.report_only
                    .insert(format!("undeclared member import: {spelled} -> {identity}"));
            }
            return self
                .disk_edge(&source)
                .unwrap_or(Resolved::Unresolvable(String::new()));
        }
        // The target member carries no answer: the legacy arms keep their
        // round-4 shape (record: the P-CTX verdict branches fire ONLY
        // where the context-free law resolves nothing).
        if declared_in.is_some() {
            let edge = member_entry(member_dir).and_then(|target| self.disk_edge_abs(&target));
            if let Some(edge) = edge {
                self.report_only.insert(format!(
                    "workspace link missing: {identity} — absent from disk"
                ));
                return edge;
            }
            return Resolved::Unresolvable(String::new());
        }
        let hoisted = super::workspace::root_lockfile_hoists(registry_root)
            && specifier == identity;
        if hoisted {
            let edge = member_entry(member_dir).and_then(|target| self.disk_edge_abs(&target));
            if let Some(edge) = edge {
            let spelled = relative(importing, &self.root)
                .unwrap_or_else(|| importing.to_string_lossy().replace('\\', "/"));
                self.report_only
                    .insert(format!("undeclared member import: {spelled} -> {identity}"));
                return edge;
            }
        }
        if let Some(hit) = self.gensrc_absent(&member_dir.join("index")) {
            return self.reported_generated(&hit, specifier);
        }
        Resolved::Unresolvable(String::new())
    }

    /// The member dir owning `dir` (deepest first), joined with the `#`
    /// spelling's remainder — the R12 evaluation root for imports misses.
    fn hash_base(&self, dir: &Path, specifier: &str) -> Option<PathBuf> {        let remainder = specifier.trim_start_matches('#');
        let mut best: Option<&Path> = None;
        for registry in &self.registries {
            for member in &registry.members {
                if dir.starts_with(&member.dir)
                    && best.is_none_or(|seen| member.dir.starts_with(seen))
                {
                    best = Some(&member.dir);
                }
            }
        }
        best.map(|dir| dir.join(remainder))
    }

    /// The owning manifest's `imports` target for the spelled `#key` —
    /// the R12 evidence root when the key maps AWAY from the member-root
    /// name join (the imports-map miss shape). Nearest package.json wins.
    fn imports_map_base(&self, dir: &Path, specifier: &str) -> Option<PathBuf> {
        let mut current = Some(dir);
        while let Some(candidate) = current {
            let manifest = candidate.join("package.json");
            if manifest.is_file() {
                let bytes = std::fs::read(&manifest).ok()?;
                let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
                let target = value
                    .get("imports")?
                    .as_object()?
                    .get(specifier)?
                    .as_str()?;
                return Some(candidate.join(target.trim_start_matches("./")));
            }
            current = candidate.parent();
        }
        None
    }

    /// The edge home of a found file: repo-relative, exclusion-checked.
    fn disk_edge(&self, found: &Path) -> Option<Resolved> {
        self.disk_edge_abs(found)
    }

    fn disk_edge_abs(&self, found: &Path) -> Option<Resolved> {
        let rel = relative(found, &self.root)?;
        if is_excluded(Path::new(&rel), self.declared.as_ref()) {
            return None;
        }
        Some(Resolved::Edge(rel))
    }

    /// The R12 generated-source test for an absent target BASE: when the
    /// extension/index table finds nothing existing, the VERBATIM spelled
    /// path ignored by the committed `.gitignore` files is a repo-declared
    /// generated artifact, not a defect. Only the verbatim spelling carries
    /// evidence — an extension-table probe would let a transpiled-output
    /// pattern (`src/**/*.js`) silence unrelated TS misses. Returns the
    /// deciding file and line.
    fn gensrc_absent(&self, base: &Path) -> Option<IgnoreHit> {
        if let Some(cached) = self.gensrc.borrow().get(base) {
            return cached.clone();
        }
        let verdict = if member_candidates(base).is_some() {
            None
        } else {
            self.gitignore.is_ignored(base, false)
        };
        self.gensrc
            .borrow_mut()
            .insert(base.to_path_buf(), verdict.clone());
        verdict
    }

    /// The R12 report-only line and the edge-free, external-free verdict.
    fn reported_generated(&mut self, hit: &IgnoreHit, specifier: &str) -> Resolved {
        self.report_only.insert(format!(
            "generated source not built: {specifier} — gitignored {}:{}",
            hit.file, hit.line
        ));
        Resolved::Reported
    }

    /// The R12 arm for the relative resolver (mod.rs): an absent relative
    /// target spelled repo-relative that the committed `.gitignore` files
    /// ignore is a generated artifact, not a defect.
    pub(crate) fn reported_generated_relative(
        &mut self,
        base: &Path,
        specifier: &str,
    ) -> Option<Resolved> {
        let absolute = self.root.join(base);
        let hit = self.gensrc_absent(&absolute)?;
        Some(self.reported_generated(&hit, specifier))
    }

    /// GENERATED ATTRIBUTION candidate read (workplan_archspec_generated_
    /// attribution, D1): the WINNING entry's `types`-arm spellings, capture-
    /// substituted by the SAME key match (D3, M2), expanded through the
    /// member candidate table. A `.d.ts` spelling is never a source (the
    /// `types_target` exclusion carried into the probe, M5) and a spelling
    /// the edge law refuses is no candidate (an attribution always lands an
    /// edge — never a line-less green). Exactly one existing candidate
    /// attributes; zero or many return None — the probe never resolves the
    /// import, so declaration order grants no authority to position-pick
    /// (decision-check B2 law).
    fn attributed_source(
        &self,
        member_dir: &Path,
        entry: &serde_json::Value,
        capture: Option<&str>,
    ) -> Option<PathBuf> {
        let mut spellings = Vec::new();
        types_arm_targets(entry, capture, &mut spellings);
        let mut source: Option<PathBuf> = None;
        for spelling in spellings {
            if !spelling.starts_with('.') {
                continue;
            }
            let trimmed = spelling.trim_start_matches("./");
            if trimmed.ends_with(".d.ts") {
                continue;
            }
            let Some(found) = member_candidates(&member_dir.join(trimmed)) else {
                continue;
            };
            if self.disk_edge(&found).is_none() {
                continue;
            }
            if source.is_some() {
                return None;
            }
            source = Some(found);
        }
        source
    }

    /// The static implicit sveltekit alias (R9, round-6 P-GEN-TSCONFIG ii;
    /// round-8 P-FW1 SUPERSEDES the probe pairing: THE SIGNAL IS THE KEY —
    /// `framework = "sveltekit"` at `[project]`, read through the engine
    /// seam). `$lib` and `$lib/<rest>` are answered at
    /// `<root>/src/lib/<rest>` through the candidate table. The file
    /// `svelte.config.js` is NEVER consulted: its presence was tree-probing
    /// (the discovery class the operator ruling closes), and its content
    /// override stays a named blind spot of the D4(2) vite class — archspec
    /// executes no config. The leg answers only where nothing else
    /// answered: a built tree's tsconfig `paths` map wins through the
    /// delegation.
    fn implicit_lib_edge(&self, specifier: &str) -> Option<Resolved> {
        let rest = match specifier.split_once('/') {
            Some(("$lib", rest)) => rest,
            None if specifier == "$lib" => "",
            _ => return None,
        };
        if !self.sveltekit {
            return None;
        }
        let base = if rest.is_empty() {
            self.root.join("src/lib")
        } else {
            self.root.join("src/lib").join(rest)
        };
        self.disk_edge(&member_candidates(&base)?)
    }

    /// The generated-location/not-installed pairing (round-6
    /// P-GEN-TSCONFIG i+iii): one ABSENT extends-target payload, three
    /// honest answers. A joined ABSOLUTE payload under a generated-class
    /// directory (the generated marker on the ONE DETECT_EXCLUDE_DIRS
    /// list) is a not-yet-built artifact: one report-only `generated
    /// tsconfig absent: <path>` line, exit 0. A RELATIVE payload IS the
    /// node_modules-style specifier verbatim (the crate's payload shape
    /// at this pin, mirrored by the pre-read): one report-only `tsconfig
    /// extends not installed: <specifier>` line, exit 0 — the `external
    /// not installed` precedent. Everything else returns None: the
    /// `tsconfig not found` red stays byte-identical, genuinely broken
    /// authoring only.
    fn tsconfig_absent_report_only(&mut self, attempted: &Path) -> Option<()> {
        if attempted.is_absolute() {
            let generated = attempted.components().any(|component| {
                matches!(component, Component::Normal(name)
                    if language::generated_dir(&name.to_string_lossy()))
            });
            if generated {
                self.report_only
                    .insert(format!("generated tsconfig absent: {}", attempted.display()));
                return Some(());
            }
            return None;
        }
        self.report_only.insert(format!(
            "tsconfig extends not installed: {}",
            attempted.display()
        ));
        Some(())
    }

    /// Drain the bare-category findings (plan: findings as generated).
    pub(crate) fn take_findings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.findings)
    }

    /// Drain the report-only lines (R7b: stderr via the report renderer).
    pub(crate) fn take_report_only(&mut self) -> Vec<String> {
        self.report_only.iter().cloned().collect()
    }

    /// Classify one bare or `#` specifier imported BY the file `importing`
    /// (absolute path). Relative specifiers never reach the seam.
    /// The memo (perf lane 2026-10-17, fix 4) keys on the importing dir
    /// and the spelled specifier — the pair the inner body reads; the
    /// builtins/virtual pre-checks are pure and cheap, so they ride the
    /// inner path and stay byte-identical on a replay.
    pub(crate) fn classify_bare(&mut self, importing: &Path, specifier: &str) -> Resolved {
        let key = (
            importing.parent().unwrap_or(&self.root).to_path_buf(),
            specifier.to_string(),
        );
        let hit = self.bare.borrow().get(&key).cloned();
        if let Some(verdict) = hit {
            return verdict;
        }
        let verdict = self.classify_bare_inner(importing, specifier);
        self.bare.borrow_mut().insert(key, verdict.clone());
        verdict
    }

    fn classify_bare_inner(&mut self, importing: &Path, specifier: &str) -> Resolved {
        // The query-suffix law (round 4 C): resolution runs on the BASE —
        // the external identity loses the query (`canvas-confetti?optional`
        // names `canvas-confetti`). Misses still name the FULL ORIGINAL
        // specifier: the anchor renderer reads `edge.specifier`.
        let base = strip_query(specifier);
        // Steps 1+2: the builtins law — external by fact, never delegated.
        // It runs BEFORE the virtual test: `node:` is a scheme the builtins
        // list pre-empts into the shipped external fact (N3, unchanged).
        if base.starts_with("node:") || is_builtin_spec(base) {
            return Resolved::External(package_identity(base));
        }
        // The virtual-specifier law (round 4 C): a scheme-shaped or
        // NUL-leading specifier is NEVER delegated and NEVER stripped —
        // one report-only line, no edge, no external-tier entry, exit 0.
        if is_virtual_spec(specifier) {
            self.report_only
                .insert(format!("virtual specifier not modelled: {specifier}"));
            return Resolved::Reported;
        }
        // The shape-line law (round-8 P-FW2, the L-D arm for `$` shapes):
        // a bare specifier whose first character is `$` CANNOT be an npm
        // package name (closed predicate: `$`-prefix ONLY — `@scope/name`
        // is legal, a leading `$` is not; the predicate is DISJOINT from
        // the virtual class, whose first byte must be alphabetic, F10).
        // Without the framework key the specifier KEEPS its generic
        // external-tier attribution exactly as today — the line NEVER
        // replaces attribution (F9, the laundering rejection stands) —
        // and gains ONE report-only line asking the framework question.
        // Wired inside the memoized inner body so the (importing dir,
        // spelled specifier) memo replays the pair exactly once (P-SEQ,
        // memo-key law).
        if specifier.starts_with('$') && !self.sveltekit {
            self.report_only.insert(format!(
                "specifier not a package name: {specifier} — framework conventions not configured?"
            ));
        }
        let dir = importing.parent().unwrap_or(&self.root).to_path_buf();
        let identity = package_identity(base);
        let key = owning_tsconfig(&dir, &self.root);
        if !self.entries.contains_key(&key) {
            self.build_entry(key.clone());
        }
        if matches!(self.entries.get(&key), Some(Entry::Faulted)) {
            // R9's static implicit alias rides BEFORE the faulted external
            // fact: a sveltekit tree BY THE KEY (round-8 P-FW1 — the key
            // selects this half) whose generated tsconfig is absent must
            // not silent-mistier the $lib surface.
            if let Some(edge) = self.implicit_lib_edge(base) {
                return edge;
            }
            return Resolved::External(identity);
        }

        // Guard G (step 4, non-`#` specifiers only): the FIRST existing
        // package.json that is unreadable-or-unparseable on the walks —
        // nearest manifest of D upward, and each ancestor's
        // node_modules/<pkg>/package.json.
        let guard = if base.starts_with('#') {
            None
        } else {
            self.guard_g(&dir, &identity)
        };

        let outcome = {
            let Entry::Ready { resolver, .. } =
                self.entries.get(&key).expect("entry built above is ready")
            else {
                unreachable!("faulted entries return before the delegation")
            };
            resolver.resolve_file(importing, base)
        };
        match outcome {
            Ok(resolution) => {
                let path = resolution.into_path_buf();
                let verdict = self.classify_ok(&path, guard, &identity);
                // The precedence law rides the Ok arm: a resolution that
                // landed inside the member dir is already the first-party
                // edge (case (i) — under-root answers reach the Edge home
                // in classify_ok); a resolution beside it owes the case-(ii)
                // audit when the name belongs to a declared member.
                if matches!(verdict, Resolved::External(_)) {
                    self.workspace_link_missing(&dir, &identity, &path);
                }
                verdict
            }
            Err(error) => {
                self.classify_error(&error, key, specifier, importing, guard, &identity)
            }
        }
    }

    /// The Ok arm (plan step 5): guard G fires EVEN on a successful
    /// fallback (M1), node_modules and outside-root answers take the
    /// external tier beside a nearest-manifest check, and an under-root
    /// answer contributes the edge — or names the exclusion it hit.
    fn classify_ok(&mut self, path: &Path, guard: Option<PathBuf>, identity: &str) -> Resolved {
        if let Some(manifest) = guard {
            self.push_manifest_invalid(&manifest);
        }
        if path_has_node_modules(path) {
            self.check_nearest_manifest(path);
            return Resolved::External(identity.to_string());
        }
        if path.starts_with(&self.root) {
            let Some(rel) = relative(path, &self.root) else {
                return Resolved::External(identity.to_string());
            };
            if is_excluded(Path::new(&rel), self.declared.as_ref()) {
                return Resolved::Unresolvable(format!(
                    " — resolves only into excluded path {}",
                    exclusion_hit(Path::new(&rel), self.declared.as_ref())
                ));
            }
            return Resolved::Edge(rel);
        }
        self.check_nearest_manifest(path);
        Resolved::External(identity.to_string())
    }

    /// The exhaustive taxonomy (plan §Taxonomy): every `ResolveError`
    /// variant, exactly one home. `ResolveError` is `#[non_exhaustive]`, so
    /// the named arms are exhaustive for the pinned 11.24.3 list and the
    /// final arm exists only to keep a future oxc addition compiling — it
    /// routes to the catch-all home, never to silence.
    fn classify_error(
        &mut self,
        error: &ResolveError,
        key: Option<PathBuf>,
        specifier: &str,
        importing: &Path,
        guard: Option<PathBuf>,
        identity: &str,
    ) -> Resolved {
        match error {
            ResolveError::NotFound(_) => {
                if let Some(manifest) = guard {
                    // Honesty law: an installed-but-broken manifest never
                    // masquerades as not-installed — never class (b).
                    self.push_manifest_invalid(&manifest);
                    return self.external_on_miss(specifier, identity);
                }
                let dir = importing.parent().unwrap_or(&self.root).to_path_buf();
                if let Some(target) = self.alias_target(&key, specifier, None) {
                    // R12 first: a gitignored alias target (the `.mercato`
                    // family — 107 reds on the pilot) is the generated
                    // class, not class (a).
                    if let Some(hit) = self.gensrc_absent(&target) {
                        return self.reported_generated(&hit, specifier);
                    }
                    // Class (a): a declared alias whose target is gone.
                    return Resolved::Unresolvable(format!(
                        " — alias target not found: {}",
                        target.display()
                    ));
                }
                if specifier.starts_with('#') {
                    // A `#name` is no package identity: never external. The
                    // R12 test runs on the member-rooted spelling (the
                    // `#generated` family — 297 reds on the pilot), then on
                    // the manifest's `imports` target for the spelled key
                    // (the key maps elsewhere — the not-built evidence
                    // lives under the MAPPED path).
                    if let Some(base) = self.hash_base(&dir, specifier)
                        && let Some(hit) = self.gensrc_absent(&base)
                    {
                        return self.reported_generated(&hit, specifier);
                    }
                    if let Some(base) = self.imports_map_base(&dir, specifier)
                        && let Some(hit) = self.gensrc_absent(&base)
                    {
                        return self.reported_generated(&hit, specifier);
                    }
                    // A `#name` is no package identity: never external.
                    return Resolved::Unresolvable(String::new());
                }
                if let Some(declared_in) = self.declared_nearest_manifest(&dir, identity) {
                    // R5 precedence law before the class-(b) home: a name
                    // the workspace registry declares as a MEMBER is a
                    // membership question, not an external-not-installed
                    // one. Round-6 P-CTX: the TARGET member's own
                    // declarations resolve it context-free; the declaration
                    // gate survives only as the report-only attribution.
                    if let Some((member_dir, registry_root)) = self.workspace_member_hit(identity)
                    {
                        return self.classify_member(
                            &member_dir,
                            &registry_root,
                            importing,
                            specifier,
                            identity,
                            Some(declared_in),
                        );
                    }
                    let spelled = manifest_spelling(&self.root, &declared_in);
                    self.report_only.insert(format!(
                        "external not installed: {identity} — declared in {spelled}, absent from disk"
                    ));
                    return Resolved::External(identity.to_string());
                }
                if let Some((member_dir, registry_root)) = self.workspace_member_hit(identity) {
                    // Round-6 P-CTX (context-free member law, UNDECLARED
                    // importer): registry MEMBERSHIP adjudicates
                    // first-party, and the TARGET member's own
                    // declarations resolve the name wherever the target
                    // carries an answer. The attribution branch keeps the
                    // round-4 narrowing: a BARE miss with NO answer under
                    // a HOISTING family rides the entry edge plus the
                    // `undeclared member import` line; under a REFUSING
                    // family it keeps the NotFound red (the runtime
                    // truth). MEMBER-NAMED SUBPATHS left the family law
                    // in round 6: they resolve under EVERY family where
                    // the target member carries the file.
                    return self.classify_member(
                        &member_dir,
                        &registry_root,
                        importing,
                        specifier,
                        identity,
                        None,
                    );
                }
                // R9's static implicit alias rides before the external
                // fallthrough: on sveltekit trees `$lib` is first-party,
                // answered archspec-side where the resolver carried no
                // map (pairing law); non-sveltekit trees never enter it.
                if let Some(edge) = self.implicit_lib_edge(strip_query(specifier)) {
                    return edge;
                }
                // The legacy external fact: a name no registry declares is
                // no membership question at all.
                Resolved::External(identity.to_string())
            }
            ResolveError::MatchedAliasNotFound(_, alias_key) => {
                // Class (a) by the payload's own key; R12 first on the
                // computed target (a gitignored target is the generated
                // class).
                if let Some(target) = self.alias_target(&key, specifier, Some(alias_key))
                    && let Some(hit) = self.gensrc_absent(&target)
                {
                    return self.reported_generated(&hit, specifier);
                }
                match self.alias_target(&key, specifier, Some(alias_key)) {
                    Some(target) => Resolved::Unresolvable(format!(
                        " — alias target not found: {}",
                        target.display()
                    )),
                    None => Resolved::Unresolvable(format!(
                        " — alias target not found for matched alias key \"{alias_key}\""
                    )),
                }
            }
            ResolveError::PackageImportNotDefined(_, manifest) => {
                // R12: when the manifest sits above a gitignored target
                // the miss is the generated class, not a defect.
                if specifier.starts_with('#')
                    && !manifest_has_imports_key(manifest, specifier)
                    && let Some(base) = manifest
                        .parent()
                        .map(|dir| dir.join(specifier.trim_start_matches('#')))
                    && let Some(hit) = self.gensrc_absent(&base)
                {
                    return self.reported_generated(&hit, specifier);
                }
                Resolved::Unresolvable(format!(" — imports map miss: {}", manifest.display()))
            }
            ResolveError::PackagePathNotExported {
                subpath,
                package_path,
                package_json_path,
                conditions,
            } => {
                // R12 first: a gitignored export-map target is the
                // generated class (the imports-map target shape rides
                // this arm when the key exists and the file does not).
                if let Some(hit) = self.gensrc_absent(package_path) {
                    return self.reported_generated(&hit, specifier);
                }
                // Class (f): the payload's ConditionNames Display rides
                // verbatim (it spells `the conditions [...]` / `no conditions`).
                Resolved::Unresolvable(format!(
                    " — {subpath} is not exported under {conditions} from package {} (see exports field in {})",
                    package_path.display(),
                    package_json_path.display()
                ))
            }
            ResolveError::TsconfigNotFound(path) => {
                if self.tsconfig_absent_report_only(path).is_none() {
                    self.push_finding(format!("tsconfig not found: {}", path.display()));
                }
                self.external_on_miss(specifier, identity)
            }
            ResolveError::TsconfigLoadFailed { path, .. } => {
                self.push_finding(format!("tsconfig load failed: {}", path.display()));
                self.external_on_miss(specifier, identity)
            }
            ResolveError::TsconfigCircularExtend(paths) => {
                // The payload Display spells the walk with " -> ".
                self.push_finding(format!(
                    "tsconfig cycle (extends or reference): {paths}"
                ));
                self.external_on_miss(specifier, identity)
            }
            ResolveError::TsconfigSelfReference(path) => {
                self.push_finding(format!(
                    "tsconfig cycle (extends or reference): {} — the project reference path points to this tsconfig",
                    path.display()
                ));
                self.external_on_miss(specifier, identity)
            }
            ResolveError::Json(json) => {
                if self.push_json_fault(json) {
                    self.external_on_miss(specifier, identity)
                } else {
                    self.push_finding(unclassified_finding(error));
                    self.external_on_miss(specifier, identity)
                }
            }
            ResolveError::Builtin { .. } => {
                // Unreachable with the pre-empting list; names itself
                // external per N3 shape (ii) either way.
                Resolved::External(identity.to_string())
            }
            // The catch-all home: class-less variants — each reddens, never
            // silences. IOError rides it too (the payload carries no usable
            // path at this pin; guard G owns the manifest cases).
            ResolveError::Ignored(_)
            | ResolveError::IOError(_)
            | ResolveError::ExtensionAlias(..)
            | ResolveError::Specifier(_)
            | ResolveError::PathNotSupported(_)
            | ResolveError::InvalidModuleSpecifier(..)
            | ResolveError::InvalidPackageTarget(..)
            | ResolveError::InvalidPackageConfig(_)
            | ResolveError::InvalidPackageConfigDefault(_)
            | ResolveError::InvalidPackageConfigDirectory(_)
            | ResolveError::Unimplemented(_)
            | ResolveError::Recursion => {
                self.push_finding(unclassified_finding(error));
                self.external_on_miss(specifier, identity)
            }
            _ => {
                // `ResolveError` is #[non_exhaustive]: an upstream addition
                // lands in the same named catch-all, never silence.
                self.push_finding(unclassified_finding(error));
                self.external_on_miss(specifier, identity)
            }
        }
    }

    /// Build the entry for one owning config: the pre-read first (R7c — the
    /// fault never reaches a delegation), then the engine.
    fn build_entry(&mut self, key: Option<PathBuf>) {
        match preload_tsconfigs(key.as_deref()) {
            Ok(preloaded) => {
                // The delegation names the entry config with the preload's
                // canonical spelling (the R7c dedupe identity); the owned
                // key rides along when the walk somehow yielded no member.
                let discovery = key.as_ref().and_then(|config_file| {
                    preloaded
                        .first()
                        .map(|member| member.path.clone())
                        .or_else(|| Some(config_file.clone()))
                });
                let tsconfig = discovery.map(|config_file| {
                    TsconfigDiscovery::Manual(TsconfigOptions {
                        config_file,
                        references: TsconfigReferences::Disabled,
                    })
                });
                let options = ResolveOptions {
                    tsconfig,
                    condition_names: CONDITION_NAMES.iter().map(|name| (*name).to_string()).collect(),
                    extensions: RESOLVE_EXTENSIONS.iter().map(|ext| (*ext).to_string()).collect(),
                    ..ResolveOptions::default()
                };
                self.entries.insert(
                    key,
                    Entry::Ready {
                        resolver: Box::new(Resolver::new(options)),
                        configs: preloaded.into_iter().map(|member| member.tsconfig).collect(),
                    },
                );
            }
            Err(fault) => {
                // The fault names itself once (dedup by message); later
                // misses on this entry state the fact without repeating.
                // The generated-location/not-installed pairing (round-6
                // P-GEN-TSCONFIG i+iii) is offered to the absent-extends
                // class FIRST; only an unpaired absence names the red.
                // The entry stays Faulted either way — a config the
                // preload could not read never falls back to alias-free
                // resolution (the R7c honesty law).
                let report_only = match &fault {
                    TsconfigPreloadError::ExtendsNotFound { attempted, .. } => {
                        self.tsconfig_absent_report_only(attempted)
                    }
                    TsconfigPreloadError::LoadFailed { .. }
                    | TsconfigPreloadError::ExtendsCycle { .. } => None,
                };
                if report_only.is_none() {
                    self.push_finding(fault.to_string());
                }
                self.entries.insert(key, Entry::Faulted);
            }
        }
    }

    /// Guard G's predicate through the P-PAR3 verdict memo (the field's
    /// law). A TRUE verdict is itself existence evidence — the predicate
    /// probes existence first — which callers may exploit only in that
    /// direction.
    fn manifest_unreadable(&mut self, path: &Path) -> bool {
        if let Some(&verdict) = self.manifest_verdicts.borrow().get(path) {
            return verdict;
        }
        let verdict = manifest_unreadable(path);
        self.manifest_verdicts
            .borrow_mut()
            .insert(path.to_path_buf(), verdict);
        verdict
    }

    /// Guard G for (dir, package): cached walk over the nearest manifests
    /// and the package's node_modules manifests on every ancestor.
    fn guard_g(&mut self, dir: &Path, package: &str) -> Option<PathBuf> {
        let cache_key = (dir.to_path_buf(), package.to_string());
        if let Some(answer) = self.guard_cache.get(&cache_key) {
            return answer.clone();
        }
        let mut answer = None;
        let mut current = Some(dir);
        while let Some(candidate_dir) = current {
            for candidate in [
                candidate_dir.join("package.json"),
                candidate_dir.join("node_modules").join(package).join("package.json"),
            ] {
                if self.manifest_unreadable(&candidate) {
                    answer = Some(candidate);
                    break;
                }
            }
            if answer.is_some() {
                break;
            }
            current = candidate_dir.parent();
        }
        self.guard_cache.insert(cache_key, answer.clone());
        answer
    }

    /// The nearest-manifest check of a successful external resolution: the
    /// first EXISTING package.json upward from the resolved file, reddening
    /// when it is unreadable-or-unparseable (dedup by path).
    fn check_nearest_manifest(&mut self, path: &Path) {
        let mut current = path.parent();
        while let Some(dir) = current {
            let manifest = dir.join("package.json");
            if self.manifest_unreadable(&manifest) {
                // Verdict TRUE implies exists() was true (the predicate's
                // own first probe): the same path the exists() check
                // below takes, with the invalid-manifest finding added.
                self.push_manifest_invalid(&manifest);
                return;
            }
            if manifest.exists() {
                // Verdict FALSE with an existing file: a readable
                // manifest — nothing to report, the walk stops.
                return;
            }
            current = dir.parent();
        }
    }

    /// The nearest package.json's declared-name answer for `identity`:
    /// dependencies ∪ devDependencies ∪ peerDependencies ∪
    /// optionalDependencies; a missing or unreadable manifest is not
    /// declared. Returns the manifest path when declared (the report line
    /// names it).
    fn declared_nearest_manifest(&mut self, dir: &Path, identity: &str) -> Option<PathBuf> {
        let mut current = Some(dir);
        let manifest = loop {
            let candidate_dir = current?;
            let candidate = candidate_dir.join("package.json");
            if candidate.is_file() {
                break candidate;
            }
            current = candidate_dir.parent();
        };
        let names = self.declared_cache.entry(manifest.clone()).or_insert_with(|| {
            manifest
                .to_str()
                .and_then(|text| std::fs::read_to_string(text).ok())
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                .map(|value| {
                    [
                        "dependencies",
                        "devDependencies",
                        "peerDependencies",
                        "optionalDependencies",
                    ]
                    .iter()
                    .filter_map(|field| value.get(*field).and_then(serde_json::Value::as_object))
                    .flat_map(|map| map.keys().cloned())
                    .collect()
                })
                .unwrap_or_default()
        });
        if names.contains(identity) {
            Some(manifest)
        } else {
            None
        }
    }

    /// The message-only alias lookup of the entry's preloaded configs
    /// (entry first, then closure order): the FIRST candidate path of the
    /// first config whose `compilerOptions.paths` matches — exact key
    /// first, else a `*`-terminated key whose prefix starts the specifier,
    /// longest match wins. Base: the declaring config's `baseUrl` joined
    /// onto its directory when relative, the config directory when absent.
    /// `match_key` (the `MatchedAliasNotFound` payload) pins the key.
    fn alias_target(&self, key: &Option<PathBuf>, specifier: &str, match_key: Option<&str>) -> Option<PathBuf> {
        // The entry map is consulted read-only; a Faulted entry never gets
        // here (the caller returns the fact first).
        let configs = match self.entries.get(key) {
            Some(Entry::Ready { configs, .. }) => configs,
            _ => return None,
        };
        alias_target_of(configs, specifier, match_key)
    }

    /// A finding line, deduped by message (the entry-fault law: the same
    /// fault names itself once, not once per file).
    fn push_finding(&mut self, line: String) {
        if self.seen_faults.insert(line.clone()) {
            self.findings.push(line);
        }
    }

    /// The `package manifest invalid` finding, deduped by path.
    fn push_manifest_invalid(&mut self, manifest: &Path) {
        if self.seen_manifests.insert(manifest.to_path_buf()) {
            self.push_finding(format!("package manifest invalid: {}", manifest.display()));
        }
    }

    /// The `Json` variant's homes: package.json → manifest invalid; a
    /// tsconfig-named file → load failed; anything else → the caller rides
    /// the catch-all. True when a home was taken.
    fn push_json_fault(&mut self, json: &JSONError) -> bool {
        let name = json.path.file_name().map(|n| n.to_string_lossy().to_lowercase());
        match name.as_deref() {
            Some("package.json") => {
                self.push_manifest_invalid(&json.path);
                true
            }
            Some(file) if file.starts_with("tsconfig") => {
                self.push_finding(format!("tsconfig load failed: {}", json.path.display()));
                true
            }
            _ => false,
        }
    }

    /// The miss verdict a class-(c/e/f-classless) finding leaves behind:
    /// the external fact — except `#` specifiers, which name no package
    /// identity and so fail loud on their own line.
    fn external_on_miss(&mut self, specifier: &str, identity: &str) -> Resolved {
        if specifier.starts_with('#') {
            Resolved::Unresolvable(String::new())
        } else {
            Resolved::External(identity.to_string())
        }
    }

    /// The manifest of a member dir, None when absent or unparseable.
    /// Perf lane 2026-10-17, fix 3 memo: one parse per member dir per scan
    /// (measured 24,293 manifest reads on a large monorepo resolve). The
    /// memo is keyed by the member DIR, so two members with DIFFERENT
    /// manifests never share a parse. A dir whose manifest exists but
    /// fails to PARSE is never memoized — the re-read keeps the exact
    /// fail-open shape of the pre-fix read (invalid-json is not a stable
    /// fact of the path).
    fn member_manifest(&self, dir: &Path) -> Option<Arc<serde_json::Value>> {
        if let Some(cached) = self.member_manifests.borrow().get(dir) {
            return cached.as_ref().map(Arc::clone);
        }
        let manifest_path = dir.join("package.json");
        let verdict = match std::fs::read(&manifest_path) {
            Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .map(Arc::new),
            Err(_) => None,
        };
        // Absence (missing file) IS a stable fact; a parse failure is not
        // (the file exists — a later read could hit a different parse of
        // changed bytes on mutable trees). Memoize absence and success,
        // never the parse-failure None.
        if verdict.is_some() || !manifest_path.exists() {
            self.member_manifests
                .borrow_mut()
                .insert(dir.to_path_buf(), verdict.as_ref().map(Arc::clone));
        }
        verdict
    }

    /// The P-DIST source answer of a member: the TS derivation table —
    /// types-condition first, then the declared target with the build-output
    /// prefix stripped plus the extension/index table. The second element is
    /// the ABSENT declared target (the `workspace target not built` carrier).
    fn member_source(&self, dir: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
        let manifest = self.member_manifest(dir)?;
        if let Some(spelled) = types_target(&manifest)
            && let Some(found) = member_candidates(&dir.join(spelled.trim_start_matches("./")))
        {
            return Some((found, None));
        }
        let spelled = declared_target(&manifest)?;
        let literal = dir.join(spelled.trim_start_matches("./"));
        if literal.is_file() && !under_build_output(&spelled) {
            return Some((literal, None));
        }
        let stripped = if under_build_output(&spelled) {
            let tail = spelled.trim_start_matches("./");
            PathBuf::from(tail.split_once('/').map_or("", |(_, tail)| tail))
        } else {
            PathBuf::from(spelled.trim_start_matches("./"))
        };
        let found = member_candidates(&dir.join(stripped))?;
        let not_built = (!literal.is_file()).then_some(literal);
        Some((found, not_built))
    }
}

/// The entry file of a workspace member: the manifest `main` (with the ts
/// extension inference when spelled extension-less), else the index
/// spellings under the member dir. This is what R5 case (iii)'s first-party
/// edge lands on when no on-disk link carried the resolver there.
fn member_entry(dir: &Path) -> Option<PathBuf> {
    let manifest = std::fs::read(dir.join("package.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())?;
    if let Some(main) = manifest.get("main").and_then(|v| v.as_str()) {
        let base = dir.join(main.trim_start_matches("./"));
        if base.is_file() {
            return Some(base);
        }
        if let Some(found) = member_candidates(&base) {
            return Some(found);
        }
    }
    member_candidates(&dir.join("index"))
}

/// Extension inference for a member-internal target: the exact path when it
/// exists, then the ts source extensions, then the index law.
fn member_candidates(base: &Path) -> Option<PathBuf> {
    if base.is_file() {
        return Some(base.to_path_buf());
    }
    for extension in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts", "svelte"] {
        let mut with_extension = base.as_os_str().to_os_string();
        with_extension.push(".");
        with_extension.push(extension);
        let candidate = PathBuf::from(&with_extension);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    for extension in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "svelte"] {
        let index = base.join(format!("index.{extension}"));
        if index.is_file() {
            return Some(index);
        }
    }
    None
}




/// The exports/imports `types`-condition TS target (P-DIST derivation arm
/// 1): the entry `.` target whose `types` condition names a TS SOURCE file
/// (`.d.ts` declaration outputs are not sources).
fn types_target(manifest: &serde_json::Value) -> Option<String> {
    let entry = manifest.get("exports")?;
    let value = match entry {
        serde_json::Value::Object(map) => map.get(".").or_else(|| map.get("*")),
        _ => None,
    }?;
    let target = match value {
        serde_json::Value::Object(conditions) => conditions.get("types"),
        _ => None,
    }?;
    let spelled = target.as_str()?;
    let trimmed = spelled.trim_start_matches('.');
    if trimmed.ends_with(".d.ts") {
        return None;
    }
    ["ts", "mts", "cts"]
        .iter()
        .any(|extension| trimmed.ends_with(&format!(".{extension}")))
        .then(|| spelled.to_string())
}

/// The declared default target: `exports` default, else `module`, else
/// `main` (P-DIST derivation arm 2 spelling).
fn declared_target(manifest: &serde_json::Value) -> Option<String> {
    if let Some(exports) = manifest.get("exports") {
        let value = match exports {
            serde_json::Value::Object(map) => map.get("."),
            value @ (serde_json::Value::String(_) | serde_json::Value::Array(_)) => Some(value),
            _ => None,
        };
        if let Some(value) = value {
            let spelled = match value {
                serde_json::Value::String(text) => Some(text.to_string()),
                serde_json::Value::Object(map) => {
                    map.get("default").and_then(|v| v.as_str()).map(str::to_string)
                }
                serde_json::Value::Array(items) => {
                    items.iter().find_map(serde_json::Value::as_str).map(str::to_string)
                }
                _ => None,
            };
            if spelled.is_some() {
                return spelled;
            }
        }
    }
    ["module", "main"]
        .iter()
        .find_map(|field| manifest.get(*field).and_then(|v| v.as_str()))
        .map(str::to_string)
}

/// True when a spelled target begins with a build-output directory.
fn under_build_output(spelled: &str) -> bool {
    let head = spelled.trim_start_matches("./").split('/').next().unwrap_or("");
    matches!(head, "dist" | "build" | "out" | "output")
}

/// The TARGET member's exports-map answer for a member-named SUBPATH
/// (P-CTX derivation, record R6: the exports map under the R3 condition
/// set — the `types` condition NEVER resolves a runtime import, and the
/// resolver adds `default` implicitly). A `*` in a key captures the
/// spelled remainder; the LONGEST matching key prefix wins (Node's law).
/// The tuple carries the WINNING ENTRY VALUE plus the capture so the
/// generated-attribution probe reads the SAME match — one key-match,
/// never a second lookup (decision-check M2).
fn exports_subpath_target(
    manifest: &serde_json::Value,
    subpath: &str,
) -> Option<(String, serde_json::Value, Option<String>)> {
    let map = manifest.get("exports")?.as_object()?;
    if let Some(value) = map.get(&format!("./{subpath}"))
        && let Some(target) = condition_target(value)
    {
        return Some((target, value.clone(), None));
    }
    let mut best: Option<(usize, String, serde_json::Value, String)> = None;
    for (key, value) in map {
        let Some(pattern) = key.strip_prefix("./") else {
            continue;
        };
        let Some((head, tail)) = pattern.split_once('*') else {
            continue;
        };
        if subpath.len() < head.len() + tail.len()
            || !subpath.starts_with(head)
            || !subpath.ends_with(tail)
        {
            continue;
        }
        let Some(target) = condition_target(value) else {
            continue;
        };
        let captured = &subpath[head.len()..subpath.len() - tail.len()];
        if best.as_ref().is_none_or(|(seen, ..)| head.len() > *seen) {
            best = Some((
                head.len(),
                target.replace('*', captured),
                value.clone(),
                captured.to_string(),
            ));
        }
    }
    best.map(|(_, target, entry, capture)| (target, entry, Some(capture)))
}

/// The `types`-arm spellings of one exports/imports VALUE, collected along
/// the SAME descent `condition_target` walks — the entry the runtime law
/// selects is the entry the probe reads (decision-check M2): the value's
/// own `types` key (string, ordered array, or nested conditions), then a
/// descent through the first present R3 condition plus `default`. A `*` in
/// a spelling receives the key's capture; `capture` is `None` for an exact
/// key, where no `*` exists by construction.
fn types_arm_targets(value: &serde_json::Value, capture: Option<&str>, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(conditions) => {
            if let Some(types) = conditions.get("types") {
                match types {
                    serde_json::Value::String(text) => out.push(substitute_capture(text, capture)),
                    serde_json::Value::Array(items) => out.extend(
                        items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(|text| substitute_capture(text, capture)),
                    ),
                    nested => types_arm_targets(nested, capture, out),
                }
            }
            for name in CONDITION_NAMES.iter().chain(std::iter::once(&"default")) {
                if let Some(next) = conditions.get(*name) {
                    types_arm_targets(next, capture, out);
                    break;
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                types_arm_targets(item, capture, out);
            }
        }
        _ => {}
    }
}

/// The capture substitution of the SAME key match (plan D3): a spelling
/// carries the key's `*`; an exact-key read spells the final path.
fn substitute_capture(spelled: &str, capture: Option<&str>) -> String {
    match capture {
        Some(capture) => spelled.replace('*', capture),
        None => spelled.to_string(),
    }
}

/// One exports/imports value under the R3 condition set: a string IS the
/// target; an object picks the FIRST condition present from the R3 set
/// plus `default` and descends; an array is an ordered fallback list.
fn condition_target(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Object(conditions) => CONDITION_NAMES
            .iter()
            .chain(std::iter::once(&"default"))
            .find_map(|name| conditions.get(*name).and_then(condition_target)),
        serde_json::Value::Array(items) => items.iter().find_map(condition_target),
        _ => None,
    }
}

/// True when the manifest's `imports` map defines the spelled `#key`.
fn manifest_has_imports_key(manifest: &Path, specifier: &str) -> bool {
    std::fs::read(manifest)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| {
            value
                .get("imports")
                .and_then(|imports| imports.as_object())
                .map(|imports| imports.contains_key(specifier))
        })
        .unwrap_or(false)
}

/// The catch-all finding: the variant's Debug form verbatim names the
/// payload, so an upstream enum addition surfaces here, never silence.
fn unclassified_finding(error: &ResolveError) -> String {
    format!("unclassified resolver error: {error:?}")
}

/// The alias map's first candidate (plan §Alias map): configs in entry
/// first, then closure order; exact key wins, else the longest
/// `*`-terminated key whose prefix starts the specifier; the first target
/// of the match rides the wildcard remainder; the base follows the
/// declaring config (`baseUrl` joined onto its dir, or the dir).
fn alias_target_of(
    configs: &[TsConfig],
    specifier: &str,
    match_key: Option<&str>,
) -> Option<PathBuf> {
    for config in configs {
        let Some(paths) = &config.compiler_options.paths else {
            continue;
        };
        let base = alias_base(config);
        let found: Option<(Option<&str>, &Vec<PathBuf>)> = if let Some(key) = match_key {
            paths.get(key).map(|targets| {
                match key.strip_suffix('*') {
                    Some(prefix) => (Some(prefix), targets),
                    None => (None, targets),
                }
            })
        } else if let Some(targets) = paths.get(specifier) {
            Some((None, targets))
        } else {
            paths
                .iter()
                .filter_map(|(key, targets)| {
                    let prefix = key.strip_suffix('*')?;
                    specifier.strip_prefix(prefix).map(|_| (Some(prefix), targets))
                })
                .max_by_key(|(prefix, _)| prefix.map_or(0, str::len))
        };
        if let Some((prefix, targets)) = found {
            let Some(target) = targets.first() else {
                continue;
            };
            let remainder = prefix.map_or("", |prefix| specifier.strip_prefix(prefix).unwrap_or(""));
            let spelled = target.to_string_lossy().replace('*', remainder);
            return Some(join_clean(&base, Path::new(&spelled)));
        }
    }
    None
}

/// The alias base of one config: its `baseUrl` joined onto the config's
/// directory when relative, the config directory when absent.
fn alias_base(config: &TsConfig) -> PathBuf {
    match &config.compiler_options.base_url {
        Some(base_url) if base_url.is_absolute() => base_url.clone(),
        Some(base_url) => join_clean(config.directory(), base_url),
        None => config.directory().to_path_buf(),
    }
}

/// `base + relative` folded component-wise (no `.` noise in the display).
fn join_clean(base: &Path, relative: &Path) -> PathBuf {
    if relative.is_absolute() {
        return relative.to_path_buf();
    }
    let mut path = base.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                path.pop();
            }
            Component::Normal(part) => path.push(part),
            _ => {}
        }
    }
    path
}

/// True when a resolved path sits inside a node_modules tree.
fn path_has_node_modules(path: &Path) -> bool {
    path.components().any(|component| component.as_os_str() == "node_modules")
}

/// Guard G's predicate: the file EXISTS yet cannot be opened, read or
/// parsed as JSON.
fn manifest_unreadable(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(file) = std::fs::File::open(path) else {
        return true;
    };
    let mut text = String::new();
    let mut reader = std::io::BufReader::new(file);
    if std::io::Read::read_to_string(&mut reader, &mut text).is_err() {
        return true;
    }
    serde_json::from_str::<serde_json::Value>(&text).is_err()
}

/// The repo-relative spelling a report line names a manifest with (the
/// model file-key law); an out-of-bound manifest spells its display path.
fn manifest_spelling(root: &Path, manifest: &Path) -> String {
    relative(manifest, root).unwrap_or_else(|| manifest.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The catch-all's byte form names the variant's Debug shape:
    /// `unclassified resolver error: <{err:?}>` (plan §Taxonomy).
    #[test]
    fn catch_all_finding_spells_the_variant_debug_form() {
        assert_eq!(
            unclassified_finding(&ResolveError::Recursion),
            "unclassified resolver error: Recursion"
        );
    }

    /// The alias map's first candidate: exact key wins over wildcard, the
    /// longest wildcard prefix wins, and the wildcard remainder rides the
    /// first target of the match (plan §Alias map).
    #[test]
    fn alias_map_first_candidate_prefers_exact_then_longest_prefix() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path().canonicalize().expect("canonical");
        write(
            &root.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@lib/*":["./src/lib/*","./alt/*"],"@lib/special":["./src/special.ts"],"@lib/special/deep/*":["./deep/*"]}}}"#,
        );
        let preloaded = preload_tsconfigs(Some(&root.join("tsconfig.json"))).expect("preload");
        let configs: Vec<TsConfig> = preloaded.into_iter().map(|member| member.tsconfig).collect();
        assert_eq!(
            alias_target_of(&configs, "@lib/special", None).as_deref(),
            Some(root.join("src/special.ts").as_path()),
            "exact key wins over wildcard"
        );
        assert_eq!(
            alias_target_of(&configs, "@lib/special/deep/x", None).as_deref(),
            Some(root.join("deep/x").as_path()),
            "longest wildcard prefix wins, first target"
        );
        assert_eq!(
            alias_target_of(&configs, "@lib/other", None).as_deref(),
            Some(root.join("src/lib/other").as_path()),
            "bare wildcard match rides the first target"
        );
        assert_eq!(
            alias_target_of(&configs, "@lib/special/deep/x", Some("@lib/special/deep/*")).as_deref(),
            Some(root.join("deep/x").as_path()),
            "a matched alias key spells its own candidate"
        );
        assert_eq!(
            alias_target_of(&configs, "other", None),
            None,
            "no alias match spells no candidate"
        );
    }

    /// The builtins list pre-empts on the first path segment; scoped
    /// packages never collide onto a built-in (N3 builtins law).
    #[test]
    fn builtins_list_preempts_the_first_segment() {
        assert!(is_builtin_spec("path"));
        assert!(is_builtin_spec("node:fs"));
        assert!(is_builtin_spec("url"));
        assert!(is_builtin_spec("assert/strict"));
        assert!(is_builtin_spec("stream/promises"));
        assert!(!is_builtin_spec("pathy"));
        assert!(!is_builtin_spec("@scope/path"));
        assert!(!is_builtin_spec("./path"));
    }

    /// The virtual-specifier law (round 4 C): scheme-shaped and NUL-leading
    /// specifiers are virtual; relative, bare, scoped and query specifiers
    /// are not. `node:` is scheme-shaped but the BUILTINS arm pre-empts it
    /// before this test runs — the predicate itself stays honest about the
    /// shape.
    #[test]
    fn virtual_predicate_names_the_scheme_and_nul_shapes() {
        assert!(super::is_virtual_spec("https://esm.sh/lodash"));
        assert!(super::is_virtual_spec("data:text/javascript,export const x = 1;"));
        assert!(super::is_virtual_spec("https://esm.sh/lodash?v=2"));
        assert!(super::is_virtual_spec("\u{0}virtual:module"));
        assert!(super::is_virtual_spec("node:fs"));
        assert!(!super::is_virtual_spec("canvas-confetti?optional"));
        assert!(!super::is_virtual_spec("./worker?worker"));
        assert!(!super::is_virtual_spec("@scope/pkg"));
        assert!(!super::is_virtual_spec("#imports"));
        assert!(!super::is_virtual_spec("C"));
        assert!(!super::is_virtual_spec(""));
    }

    /// The query-suffix law (round 4 C): resolution sees the base; the
    /// original keeps its query for the miss line.
    #[test]
    fn query_suffix_strips_for_resolution_only() {
        assert_eq!(super::strip_query("./worker?worker"), "./worker");
        assert_eq!(super::strip_query("canvas-confetti?optional"), "canvas-confetti");
        assert_eq!(super::strip_query("./plain"), "./plain");
        assert_eq!(super::strip_query("./trailing?"), "./trailing");
    }

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, content).expect("write");
    }
}
