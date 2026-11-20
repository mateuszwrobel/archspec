// The ONE tree-root-canonical unit-id resolution site (ADR-069 clause
// 4.4a(i); decision-check F-W5: the value join must consume tree-root
// canonical ids at ONE resolution site — a per-driver name reaching a row
// fires the 4.4 identity-conflict refusal on every scan).
//
// The law this module owns: every unit id inside any multi-producer fact
// identity is the TREE-ROOT-CANONICAL unit id — the canonicalized
// tree-root-relative directory of the unit that owns the fact. Two producers
// naming one node by two ecosystem spellings (a crate name, a package path,
// a bare `"."` from a build root) resolve to one id, so their rows carry one
// identity: equal identity merges (4.4, one `PLUGIN-OVERLAP` census row per
// merged identity), and only a genuinely unresolvable spelling reaches the
// named refusal. A driver's bare `"."` unit name is retired as row identity:
// it is never looked up as a spelling — it canonicalizes through the
// producer's own directory to the tree-root id owning that directory.
//
// The canonical id spelling is the tree-root-relative directory path,
// normalized by `canonical_dir`; the tree root's own unit id is `"."`. The
// retired-`"."` law binds PRODUCER spellings, never a resolved id: a resolved
// row carries the owning unit's id, and `"."` appears only when the owning
// unit IS the tree root.
//
// Generic by law (engine boundary): this file speaks only of directories,
// opaque spellings and row identities — no application, framework, ecosystem
// or vendor vocabulary, and no language driver import. It is the single
// site: fact writers (the value-join and declaration consumers) call here
// and never re-derive an id. The header stays plain `//` comments on purpose:
// the weld in `tests/events_value_join_unitid.rs` compiles this exact file
// through `include!`, which never accepts inner attributes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A tree-root-canonical unit id: the normalized tree-root-relative
/// directory of the owning unit (`"apps/desktop"`, or `"."` for the tree
/// root itself).
pub type UnitId = String;

/// Normalize a tree-root-relative path spelling: separators collapse to `/`,
/// empty and `"."` segments drop, `".."` steps up lexically (never past the
/// root spelling), and the empty result is the root `"."`. This is the only
/// path-normalization rule a unit id may pass through — callers never hand
/// an unnormalized spelling to an identity.
pub fn canonical_dir(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

/// A spelling the index could not resolve to any tree-root unit — the fail
/// -closed arm (the #56 posture): the row is NOT merged and the refusal
/// names itself with the spelling and the context it was resolved against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The unit spelling that answered nothing.
    pub spelling: String,
    /// What the resolution was anchored to — the producer's directory for a
    /// row endpoint, the path itself for [`UnitIdIndex::path_owner`]. Empty
    /// when nothing anchored the lookup.
    pub context: String,
}

impl fmt::Display for Unresolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.context.is_empty() {
            write!(
                f,
                "UNITID-UNRESOLVED unit={} — no tree-root unit answers the spelling; row not merged",
                self.spelling
            )
        } else {
            write!(
                f,
                "UNITID-UNRESOLVED unit={} at={} — no tree-root unit answers the spelling; row not merged",
                self.spelling, self.context
            )
        }
    }
}

/// The tree-root index: every unit of one tree, known by its directory and
/// by the ecosystem spellings that name it. Construction is the caller's
/// deterministic walk (model units in sorted order); the index itself is a
/// `BTree` map/set throughout, so every answer is deterministic.
#[derive(Debug, Default, Clone)]
pub struct UnitIdIndex {
    /// Canonical tree-root-relative directories of the registered units.
    dirs: BTreeSet<String>,
    /// Ecosystem spelling -> canonical unit id. A spelling is first-wins:
    /// registration order (the caller's deterministic walk) settles any
    /// duplicate claim, never map iteration luck.
    spellings: BTreeMap<String, UnitId>,
}

impl UnitIdIndex {
    /// An empty index (no unit answers anything yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one unit: its tree-root-relative directory (canonicalized on
    /// arrival) plus every ecosystem spelling that names it (crate name,
    /// package name, project name — opaque here). The retired-`"."` law
    /// applies at registration: a bare `"."` spelling never keys the table,
    /// because dot identity resolves through the producer's directory, never
    /// through a spelling lookup.
    pub fn register(&mut self, dir: &str, spellings: impl IntoIterator<Item = impl Into<String>>) {
        let id = canonical_dir(dir);
        self.dirs.insert(id.clone());
        for spelling in spellings {
            let spelling: String = spelling.into();
            if spelling == "." {
                continue;
            }
            self.spellings.entry(spelling).or_insert_with(|| id.clone());
        }
    }

    /// Resolve one producer's unit spelling to its tree-root-canonical id.
    /// Resolution order is fixed (the ONE site's law):
    /// 1. a spelling canonicalizing to `"."` is the retired driver dot — it
    ///    resolves through ownership of `producer_dir`, never through the
    ///    spelling table;
    /// 2. an exact ecosystem spelling answers its unit's id;
    /// 3. a spelling that READS as a tree-root path — it contains `/`, or it
    ///    equals a registered unit directory — answers its owning unit's id
    ///    (see [`Self::path_owner`]);
    /// 4. anything else is a fail-closed [`Unresolved`]: an unknown bare name
    ///    never falls into the root-ownership arm, so it can never be
    ///    absorbed into the root's id by silence.
    pub fn resolve(&self, spelling: &str, producer_dir: &str) -> Result<UnitId, Unresolved> {
        let canonical = canonical_dir(spelling);
        if canonical == "." {
            let anchor = canonical_dir(producer_dir);
            return self.path_owner_with_context(&anchor, &anchor, spelling);
        }
        if let Some(id) = self.spellings.get(spelling).cloned() {
            return Ok(id);
        }
        if canonical != spelling
            && let Some(id) = self.spellings.get(&canonical).cloned()
        {
            return Ok(id);
        }
        if spelling.contains('/') || self.dirs.contains(&canonical) {
            return self.path_owner_with_context(&canonical, &canonical, spelling);
        }
        Err(Unresolved {
            spelling: spelling.to_string(),
            context: canonical_dir(producer_dir),
        })
    }

    /// The D-Module face: a tree-root-relative PATH resolves to the
    /// canonical id of the unit that OWNS it — the deepest registered unit
    /// directory that contains the path (a unit's own directory contains
    /// itself; the tree root contains everything it registers under).
    pub fn path_owner(&self, path: &str) -> Result<UnitId, Unresolved> {
        let canonical = canonical_dir(path);
        self.path_owner_with_context(&canonical, &canonical, path)
    }

    /// The ownership arm shared by every resolution: the deepest registered
    /// unit directory containing `dir`. Fail-closed when no registered unit
    /// owns the directory — an untethered path never earns a guessed id.
    fn path_owner_with_context(
        &self,
        dir: &str,
        context: &str,
        spelling: &str,
    ) -> Result<UnitId, Unresolved> {
        let mut owner: Option<&String> = None;
        for candidate in &self.dirs {
            let contains = candidate == "."
                || candidate == dir
                || (dir.starts_with(candidate.as_str())
                    && dir[candidate.len()..].starts_with('/'));
            if contains && owner.is_none_or(|held| candidate.len() > held.len()) {
                owner = Some(candidate);
            }
        }
        owner.cloned().ok_or_else(|| Unresolved {
            spelling: spelling.to_string(),
            context: context.to_string(),
        })
    }
}

/// One producer's fact row BEFORE canonicalization: the identity endpoints
/// as that producer spelled them, the join-key value (4.4a(ii) — an opaque
/// string here, the writers own the convention namespace), and the tree-root
/// directory of the unit that produced the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerRow {
    /// `from` endpoint spelling as the producer named it.
    pub from: String,
    /// `to` endpoint spelling as the producer named it.
    pub to: String,
    /// The join-key value — never renamed by this module (4.4a(ii)).
    pub event: String,
    /// Tree-root-relative directory of the producing unit; the anchor that
    /// retires a bare `"."` endpoint spelling.
    pub producer_dir: String,
}

/// A multi-producer fact identity after canonicalization: every endpoint is
/// a tree-root-canonical unit id (4.4a(i)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalRow {
    pub from: UnitId,
    pub to: UnitId,
    pub event: String,
}

/// The outcome of merging one producer set: the merged canonical rows in
/// deterministic order, one `PLUGIN-OVERLAP` census line per identity that
/// more than one producer row drew (the 4.4 merge arm), and the named 4.4
/// refusals — rows that did NOT merge.
#[derive(Debug, Default)]
pub struct MergeOutcome {
    pub rows: Vec<CanonicalRow>,
    pub overlaps: Vec<String>,
    pub refusals: Vec<String>,
}

/// Canonicalize one producer row's identity at the ONE site. Both endpoints
/// resolve through [`UnitIdIndex::resolve`] against the row's producer
/// directory; every unresolved endpoint contributes its named refusal.
pub fn canonicalize_row(
    index: &UnitIdIndex,
    row: &ProducerRow,
) -> Result<CanonicalRow, Vec<String>> {
    let from = index.resolve(&row.from, &row.producer_dir);
    let to = index.resolve(&row.to, &row.producer_dir);
    match (from, to) {
        (Ok(from), Ok(to)) => Ok(CanonicalRow {
            from,
            to,
            event: row.event.clone(),
        }),
        (from, to) => {
            let mut refusals = Vec::new();
            if let Err(unresolved) = from {
                refusals.push(unresolved.to_string());
            }
            if let Err(unresolved) = to {
                refusals.push(unresolved.to_string());
            }
            Err(refusals)
        }
    }
}

/// Merge one producer set to canonical fact identity (the 4.4/4.4a arms):
/// rows whose canonical identity is equal merge to one row with one
/// `PLUGIN-OVERLAP` census line naming the merge; rows the site cannot
/// resolve are refused by name and NOT merged (fail-closed). Equal identity
/// means equal `(from, to, event)` AFTER canonicalization — two producers
/// spelling one node differently never double-count and never conflict
/// (the F-W5 proving shape). Output order is deterministic: the merge keys
/// on a `BTreeMap` over the canonical identity.
pub fn merge_rows(index: &UnitIdIndex, rows: &[ProducerRow]) -> MergeOutcome {
    let mut merged: BTreeMap<CanonicalRow, usize> = BTreeMap::new();
    let mut refusals = Vec::new();
    for row in rows {
        match canonicalize_row(index, row) {
            Ok(canonical) => *merged.entry(canonical).or_insert(0) += 1,
            Err(lines) => refusals.extend(lines),
        }
    }
    let mut outcome = MergeOutcome {
        rows: Vec::new(),
        overlaps: Vec::new(),
        refusals,
    };
    for (row, producers) in merged {
        if producers > 1 {
            outcome.overlaps.push(format!(
                "PLUGIN-OVERLAP row=({}, {}, {}) producers={} — identities equal after canonicalization, merged to one row",
                row.from, row.to, row.event, producers
            ));
        }
        outcome.rows.push(row);
    }
    outcome
}
