//! The static `.gitignore` evaluator (R12, round-7 P-GENSRC): git's ignore
//! semantics evaluated over COMMITTED `.gitignore` files only — archspec
//! NEVER spawns git. The measured reason (record R12): `git check-ignore`
//! consults `core.excludesFile` and `.git/info/exclude` (machine-dependent
//! verdicts) and exits 128 outside repos; the class must be a fact of the
//! tree, not of the machine.
//!
//! Fidelity law (R12, pinned against a dev-time `git check-ignore -v`
//! oracle — see the test module): the tree is walked from the scan root
//! down to the target's directory; each directory's `.gitignore` applies
//! its patterns to the path remainder; the DEEPEST file holding a match
//! decides, and within one file the LAST matching pattern decides; `!`
//! negates; a pattern with no slash matches its name at any depth; a
//! leading slash or a mid slash anchors the pattern to the ignoring file's
//! directory; a trailing slash matches directories only; `**` spans any
//! number of path segments; a path under an EXCLUDED parent can never be
//! re-included (git does not descend into excluded directories). Blank
//! lines and `#` comments carry no pattern.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// The one-line answer of the evaluator: which committed `.gitignore` and
/// which line within it decides the verdict (the report-only line spells
/// both).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreHit {
    /// The `.gitignore` file whose pattern decided, repo-relative spelling.
    pub file: String,
    /// The 1-based line number of the deciding pattern.
    pub line: usize,
}

/// The per-scan `.gitignore` cache (R12, perf lane 2026-10-17).
///
/// Why the cache: the generated-source test walks the ancestor chain of
/// every absent candidate, and every walk step used to RE-READ and re-parse
/// every ancestor `.gitignore` — the dominant file-open cost of a cold
/// resolve (measured ~6 opens per query on a large tree). One parse per
/// file per scan: entries are keyed by the file path, so one parse serves
/// every walk that passes the directory; a negative entry (missing or
/// unreadable file) is cached too, keeping the fail-closed no-file law
/// without a re-stat on the next walk. A `.gitignore` cannot change during
/// a scan (the scan is lexical), so the parse never invalidates.
pub(crate) struct Evaluator {
    root: PathBuf,
    files: RefCell<HashMap<PathBuf, Option<Arc<Vec<Pattern>>>>>,
}

impl Evaluator {
    /// The cache for one scan rooted at `root`.
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            files: RefCell::new(HashMap::new()),
        }
    }

    /// Same verdict as the free `is_ignored`, served from the per-scan cache.
    pub(crate) fn is_ignored(&self, path: &Path, is_dir: bool) -> Option<IgnoreHit> {
        let Ok(relative) = path.strip_prefix(&self.root) else {
            return None;
        };
        let parts: Vec<String> = relative
            .components()
            .filter_map(|part| match part {
                Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            return None;
        }
        evaluate(&self.root, &self.files, &parts, is_dir)
    }
}

/// The recursive git verdict: the PARENT directory's verdict is computed
/// first and, when the parent is excluded, NOTHING deeper can re-include
/// (git never descends into excluded directories — R12, fixture
/// GEN-PARENT-DIR-IGNORE). Otherwise every `.gitignore` from the root to
/// the path's own directory applies to the remainder, the deepest matching
/// file wins, and within one file the LAST matching line wins. Reads ride
/// the per-scan cache (see [`Evaluator`]).
fn evaluate(
    root: &Path,
    files: &RefCell<HashMap<PathBuf, Option<Arc<Vec<Pattern>>>>>,
    parts: &[String],
    is_dir: bool,
) -> Option<IgnoreHit> {
    let (ignored, hit) = check(root, files, parts, is_dir);
    ignored.then_some(())?;
    hit
}

fn check(
    root: &Path,
    files: &RefCell<HashMap<PathBuf, Option<Arc<Vec<Pattern>>>>>,
    parts: &[String],
    is_dir: bool,
) -> (bool, Option<IgnoreHit>) {
    // The no-re-entry law: an excluded parent decides, its hit line rides.
    if !parts.is_empty() {
        let (parent_ignored, parent_hit) = check(root, files, &parts[..parts.len() - 1], true);
        if parent_ignored {
            return (true, parent_hit);
        }
    }
    let mut ignored = false;
    let mut hit = None;
    for depth in 0..parts.len() {
        let dir = parts[..depth].iter().collect::<PathBuf>();
        let ignore_file = root.join(dir).join(".gitignore");
        let Some(patterns) = patterns_at(files, &ignore_file) else {
            continue;
        };
        for pattern in patterns.iter() {
            if pattern.matches(&parts[depth..], is_dir) {
                ignored = !pattern.negated;
                hit = (!pattern.negated).then(|| IgnoreHit {
                    file: repo_spelling(root, &ignore_file),
                    line: pattern.line,
                });
            }
        }
    }
    (ignored, hit)
}

/// The cached parsed rules of one `.gitignore` (`None` for a missing or
/// unreadable file — the fail-closed no-file law, cached so the next walk
/// passing this directory does not re-stat it).
fn patterns_at(
    files: &RefCell<HashMap<PathBuf, Option<Arc<Vec<Pattern>>>>>,
    ignore_file: &Path,
) -> Option<Arc<Vec<Pattern>>> {
    if let Some(cached) = files.borrow().get(ignore_file) {
        return cached.as_ref().map(Arc::clone);
    }
    let parsed = std::fs::read_to_string(ignore_file).ok().map(|text| {
        Arc::new(
            text.lines()
                .enumerate()
                .filter_map(|(index, raw)| Pattern::parse(raw, index + 1))
                .collect::<Vec<_>>(),
        )
    });
    files
        .borrow_mut()
        .insert(ignore_file.to_path_buf(), parsed.as_ref().map(Arc::clone));
    parsed
}

/// The repo-relative slash spelling a report line names a file with.
fn repo_spelling(root: &Path, path: &Path) -> String {
    let spelled = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if spelled.is_empty() {
        path.to_string_lossy().replace('\\', "/")
    } else {
        spelled
    }
}

/// One parsed `.gitignore` line (its 1-based source line rides for the
/// report — the cache parses each file once per scan).
struct Pattern {
    negated: bool,
    /// True when the pattern's shape anchors it to the ignoring file's dir.
    anchored: bool,
    /// True when the pattern matches directories only (`dir/`).
    dir_only: bool,
    /// The pattern's segments; a segment is literal glob text or `**`.
    segments: Vec<String>,
    /// The 1-based line the pattern was parsed from.
    line: usize,
}

impl Pattern {
    fn parse(raw: &str, line: usize) -> Option<Self> {
        let mut text = raw.trim_end_matches('\r');
        let negated = text.strip_prefix('!').unwrap_or(text) != text;
        if negated {
            text = text.strip_prefix('!').unwrap_or(text);
        }
        let text = text.strip_prefix('\\').unwrap_or(text);
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let dir_only = trimmed.ends_with('/');
        let body = trimmed.trim_end_matches('/');
        if body.is_empty() {
            return None;
        }
        // A slash anywhere but at the end anchors the pattern (git's rule
        // for the mid- or leading-slash shape); a bare name matches depth.
        let anchored = body.contains('/');
        let body = body.trim_start_matches('/');
        let segments = body
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect();
        Some(Self {
            negated,
            anchored,
            dir_only,
            segments,
            line,
        })
    }

    /// Matches one path remainder (the parts below the ignoring file).
    fn matches(&self, parts: &[String], is_dir: bool) -> bool {
        if self.dir_only && !is_dir {
            // A `dir/` pattern is the directory (and, by the descent law,
            // its contents); a FILE of the same name is not it.
            return false;
        }
        if self.anchored {
            return self.segments_match(&self.segments, parts);
        }
        // Unanchored: the name matches at this level or at any deeper one.
        (0..parts.len()).any(|start| self.segments_match(&self.segments, &parts[start..]))
    }

    /// Segment-wise match with `**` wildcard segments; `**` spans zero or
    /// more path segments. Ancestors are checked separately (the descent
    /// law), so the final pattern segment matches ONE part here.
    fn segments_match(&self, pattern: &[String], parts: &[String]) -> bool {
        match (pattern.first(), pattern.get(1)) {
            (None, _) => parts.is_empty(),
            (Some(segment), _) if segment == "**" => {
                (0..=parts.len()).any(|rest| self.segments_match(&pattern[1..], &parts[rest..]))
            }
            (Some(segment), _) => {
                parts
                    .first()
                    .is_some_and(|part| glob_match(segment, part))
                    && self.segments_match(&pattern[1..], &parts[1..])
            }
        }
    }
}

/// One segment's glob: `*` and `?` within one path segment, `\` escapes,
/// everything else literal.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pv: Vec<char> = pattern.chars().collect();
    let tv: Vec<char> = text.chars().collect();
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star: Option<(usize, usize)> = None;
    while ti < tv.len() {
        let pattern_char = pv.get(pi).copied();
        if pattern_char == Some('*') {
            star = Some((pi, ti));
            pi += 1;
        } else if pattern_char == Some('?') {
            pi += 1;
            ti += 1;
        } else if pattern_char == Some('\\') {
            pi += 1;
            if pv.get(pi) == tv.get(ti) {
                pi += 1;
                ti += 1;
            } else if let Some((sp, st)) = star {
                pi = sp + 1;
                star = Some((sp, st + 1));
                ti = st + 1;
            } else {
                return false;
            }
        } else if pattern_char == tv.get(ti).copied() {
            pi += 1;
            ti += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            star = Some((sp, st + 1));
            ti = st + 1;
        } else {
            return false;
        }
    }
    while pv.get(pi) == Some(&'*') {
        pi += 1;
    }
    pi == pv.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Dev-time conformance oracle (R12 fidelity law): every verdict below
    /// was captured with `git check-ignore -v` (and `-q` exit codes, the
    /// authoritative verdicts — `-v` prints the matching negation line with
    /// exit 0) against a probe tree on 2026-10-17. Git was spawned at DEV
    /// time only; this test NEVER spawns git (runtime independence law).
    fn oracle_tree() -> tempfile::TempDir {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        fs::write(
            root.join(".gitignore"),
            "generated/\n\
             .mercato\n\
             *.log\n\
             !keep.log\n\
             build/\n\
             !build/keep.ts\n\
             logs/\n\
             rootonly.txt\n\
             /packages/c/dist\n\
             packages/d/dist/*\n\
             !packages/d/dist/important.js\n\
             src/**/gen\n\
             a/**/b\n\
             **/star\n",
        )
        .expect("root gitignore");
        for dir in [
            "packages/a/generated",
            "packages/b/dist",
            "packages/c/dist",
            "packages/d/dist",
            "packages/e/generated",
            "packages/f/dist",
            "src/deep/nested",
            ".mercato",
        ] {
            fs::create_dir_all(root.join(dir)).expect("mkdir");
        }
        fs::write(root.join("packages/b/.gitignore"), "dist/\n").expect("nested b");
        fs::write(root.join("packages/e/.gitignore"), "!generated\n").expect("nested e");
        fs::write(root.join("packages/f/.gitignore"), "!dist/important.js\n").expect("nested f");
        for file in [
            "packages/a/generated/x.ts",
            ".mercato/map.js",
            "src/deep/rootonly.txt",
            "build/keep.ts",
            "app.log",
            "keep.log",
            "packages/c/dist/entry.js",
            "packages/d/dist/other.js",
            "packages/d/dist/important.js",
            "packages/e/generated/z.ts",
            "packages/f/dist/important.js",
            "src/deep/nested/gen/file.ts",
            "logs",
            "star",
            "src/star",
            "x/star.ts",
            "packages/b/dist/entry.js",
        ] {
            fs::create_dir_all(root.join(file).parent().expect("parent")).expect("mkdir");
            fs::write(root.join(file), "\n").expect("write");
        }
        tree
    }

    fn verdict(root: &Path, spelled: &str) -> Option<IgnoreHit> {
        let path = root.join(spelled);
        let is_dir = path.is_dir();
        Evaluator::new(root).is_ignored(&path, is_dir)
    }

    #[test]
    fn oracle_verdicts_reproduce_git_check_ignore() {
        let tree = oracle_tree();
        let root = tree.path();
        // Ignored by git (check-ignore -q exit 0) — the deciding root-file
        // line pinned from the oracle capture:
        let cases: [(&str, &str, usize); 10] = [
            ("packages/a/generated/x.ts", ".gitignore", 1),
            (".mercato/map.js", ".gitignore", 2),
            ("src/deep/rootonly.txt", ".gitignore", 8),
            ("build/keep.ts", ".gitignore", 5),
            ("app.log", ".gitignore", 3),
            ("packages/c/dist/entry.js", ".gitignore", 9),
            ("packages/d/dist/other.js", ".gitignore", 10),
            ("src/deep/nested/gen/file.ts", ".gitignore", 12),
            ("star", ".gitignore", 14),
            ("src/star", ".gitignore", 14),
        ];
        for (spelled, file, line) in cases {
            let hit = verdict(root, spelled);
            assert_eq!(
                hit.map(|hit| (hit.file, hit.line)),
                Some((file.to_string(), line)),
                "{spelled} must be ignored by {file}:{line}"
            );
        }
        // Ignored through the NESTED file (deeper-wins):
        let hit = verdict(root, "packages/b/dist/entry.js").expect("nested ignore");
        assert_eq!(hit.file, "packages/b/.gitignore");
        assert_eq!(hit.line, 1);
        // NOT ignored by git (check-ignore -q exit 1):
        for spelled in [
            "keep.log",
            "packages/d/dist/important.js",
            "packages/f/dist/important.js",
            "packages/e/generated/z.ts",
            "logs",
            "x/star.ts",
        ] {
            assert_eq!(verdict(root, spelled), None, "{spelled} must stay tracked");
        }
    }

    #[test]
    fn parent_dir_exclusion_blocks_reentry() {
        // GEN-PARENT-DIR-IGNORE shape: `build/` excludes the dir; the
        // later `!build/keep.ts` negation can NEVER re-include (git does
        // not descend into excluded directories).
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        fs::write(root.join(".gitignore"), "build/\n!build/keep.ts\n").expect("ignore");
        fs::create_dir_all(root.join("build")).expect("mkdir");
        fs::write(root.join("build/keep.ts"), "\n").expect("write");
        let hit = verdict(root, "build/keep.ts").expect("excluded with the parent");
        assert_eq!(hit.line, 1);
    }

    #[test]
    fn deeper_file_beats_shallower_and_last_line_wins() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        fs::write(root.join(".gitignore"), "gen\n!src/gen\n").expect("ignore");
        fs::create_dir_all(root.join("packages/p/gen")).expect("mkdir");
        fs::write(root.join("packages/p/.gitignore"), "gen\n").expect("nested");
        fs::write(root.join("packages/p/gen/a.ts"), "\n").expect("write");
        // The nested file re-states `gen` — same verdict, its own line.
        let hit = verdict(root, "packages/p/gen/a.ts").expect("ignored");
        assert_eq!(hit.file, "packages/p/.gitignore");
        assert_eq!(hit.line, 1);
        // Within one file the LAST match decides: `!src/gen` negates the
        // shallower `gen` for the src/ tree.
        fs::create_dir_all(root.join("src/gen")).expect("mkdir3");
        fs::write(root.join("src/gen/b.ts"), "\n").expect("write");
        assert_eq!(verdict(root, "src/gen/b.ts"), None);
    }

    #[test]
    fn anchored_dir_only_and_doublestar_shapes() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        fs::write(
            root.join(".gitignore"),
            "/only-root\nmid/file.txt\na/**/b\n",
        )
        .expect("ignore");
        fs::create_dir_all(root.join("deep/only-root")).expect("mkdir");
        fs::write(root.join("only-root"), "\n").expect("write");
        fs::write(root.join("deep/only-root/x"), "\n").expect("nope");
        fs::create_dir_all(root.join("mid")).expect("mkdir2");
        fs::write(root.join("mid/file.txt"), "\n").expect("write2");
        fs::create_dir_all(root.join("sub/mid")).expect("mkdir3");
        fs::write(root.join("sub/mid/file.txt"), "\n").expect("write3");
        fs::create_dir_all(root.join("a/x/y/b")).expect("mkdir4");
        fs::write(root.join("a/x/y/b/c.ts"), "\n").expect("write4");
        assert!(verdict(root, "only-root").is_some(), "anchored root match");
        assert_eq!(
            verdict(root, "deep/only-root"),
            None,
            "an anchored pattern never depths"
        );
        assert!(
            verdict(root, "mid/file.txt").is_some(),
            "a mid-slash pattern anchors to the file's dir"
        );
        assert_eq!(
            verdict(root, "sub/mid/file.txt"),
            None,
            "a mid-slash pattern in one file never depths below that file's dir"
        );
        assert!(
            verdict(root, "a/x/y/b/c.ts").is_some(),
            "`a/**/b` spans segments"
        );
    }
}
