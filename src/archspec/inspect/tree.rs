use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct Dir {
    pub files: BTreeSet<String>,
    pub dirs: BTreeMap<String, Dir>,
}

pub fn build_tree(nodes: &BTreeSet<String>) -> Dir {
    let mut root = Dir {
        files: BTreeSet::new(),
        dirs: BTreeMap::new(),
    };
    for node in nodes {
        let path = Path::new(node);
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let mut cur = &mut root;
        for component in parent.components() {
            let name = component.as_os_str().to_string_lossy().into_owned();
            cur = cur.dirs.entry(name).or_insert_with(|| Dir {
                files: BTreeSet::new(),
                dirs: BTreeMap::new(),
            });
        }
        cur.files.insert(node.clone());
    }
    root
}
