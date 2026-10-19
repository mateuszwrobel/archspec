//! The bounded tsconfig owning chain (ADR-024, resolution record R2).
//!
//! An importing file's owning config is found by walking UPWARD from the
//! importing file's directory: the first directory holding a `tsconfig.json`
//! owns the file, nearest directory first. The walk is BOUNDED BY THE SCAN
//! ROOT — it consults the root's own directory and stops there, never past
//! it (R2, the discovery bound). A config above the scan root is never read
//! and never reddened, so a parent tree's fault cannot poison a subtree
//! verdict. Where no config exists at-or-above the file within the bound the
//! file lives in the inferred project — alias-free and honest: tsconfig
//! ABSENCE is not a fault class, so absence yields no finding at all.
//!
//! This is archspec's side of the owning-chain computation. The pinned
//! crate's `Auto` discovery walks unbounded and silently skips unreadable
//! candidates, so the driver never delegates discovery to it; it computes
//! the chain here and hands the result to the pre-read leg (`config_preload`)
//! before delegation (the never-silent law, resolution record R7c).

use std::path::{Path, PathBuf};

/// The config file name the upward walk consults — the discovery candidate,
/// spelled exactly as the pinned crate's own upward walk spells it.
pub(crate) const TSCONFIG_FILE: &str = "tsconfig.json";

/// The nearest owning `tsconfig.json` for an importing file in `file_dir`,
/// walking upward from `file_dir` and consulting `scan_root` itself as the
/// last directory on the walk.
///
/// `Some(path)` names the owning config. `None` is the inferred project —
/// alias-free, never a fault — either because no config exists within the
/// bound, or because `file_dir` lies outside `scan_root` altogether, where
/// consulting any directory would break the bound. Directories past the scan
/// root are never consulted: neither stat'ed nor read.
pub(crate) fn owning_tsconfig(file_dir: &Path, scan_root: &Path) -> Option<PathBuf> {
    let mut dir = Some(file_dir);
    while let Some(current) = dir {
        if !current.starts_with(scan_root) {
            // Past the bound: out-of-bound ancestors are never consulted.
            return None;
        }
        let candidate = current.join(TSCONFIG_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if current == scan_root {
            // The bound includes the scan root and ends there.
            return None;
        }
        dir = current.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::owning_tsconfig;
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
        fn write(&self, rel: &str, content: &str) {
            let path = self.root().join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, content).expect("write");
        }
    }

    #[test]
    fn nearest_config_on_the_walk_wins() {
        let tree = Tree::new();
        tree.write("tsconfig.json", "{}");
        tree.write("packages/app/tsconfig.json", "{}");
        let file_dir = tree.root().join("packages/app/src");
        assert_eq!(
            owning_tsconfig(&file_dir, &tree.root()),
            Some(tree.root().join("packages/app/tsconfig.json"))
        );
    }

    #[test]
    fn scan_root_config_owns_files_with_nothing_nearer() {
        let tree = Tree::new();
        tree.write("tsconfig.json", "{}");
        let file_dir = tree.root().join("packages/app/src");
        assert_eq!(
            owning_tsconfig(&file_dir, &tree.root()),
            Some(tree.root().join("tsconfig.json"))
        );
    }

    #[test]
    fn inferred_project_when_no_config_within_bound() {
        let tree = Tree::new();
        let file_dir = tree.root().join("packages/app/src");
        fs::create_dir_all(&file_dir).expect("mkdir");
        assert_eq!(owning_tsconfig(&file_dir, &tree.root()), None);
    }

    #[test]
    fn out_of_bound_ancestor_config_is_never_consulted() {
        // A broken config one level ABOVE the scan root (the monorepo-parent
        // case) must leave the subtree alias-free and green, never red.
        let outer = tempfile::tempdir().expect("tempdir");
        let root = outer.path().join("child-tree");
        fs::create_dir_all(root.join("src")).expect("mkdir");
        fs::write(root.join("src/file.ts"), "").expect("write");
        fs::write(
            outer.path().join("tsconfig.json"),
            r#"{"extends": "./broken-parent-fault.json"}"#,
        )
        .expect("write");
        assert_eq!(owning_tsconfig(&root.join("src"), &root), None);
    }

    #[test]
    fn directory_outside_the_bound_is_never_consulted() {
        let outer = tempfile::tempdir().expect("tempdir");
        let root = outer.path().join("scanned");
        let outside = outer.path().join("elsewhere");
        fs::create_dir_all(&root).expect("mkdir");
        fs::create_dir_all(&outside).expect("mkdir");
        fs::write(outside.join("tsconfig.json"), "{}").expect("write");
        // The file's own dir is not under the scan root: nothing is read,
        // the answer is the inferred project, never the config outside.
        assert_eq!(owning_tsconfig(&outside, &root), None);
    }
}
