//! The deterministic clustering overlay's computation (lane A).
//!
//! Pure functional: a prepared graph goes in, assignments, counts and one
//! structure label come out. No I/O, no state, no lifecycle — a deterministic
//! computation has no domain objects to model (workplan
//! `archspec_inspect_clusters` §Modularity). The contract lives in the
//! clustering design contracts, §4–§5; the
//! algorithm is greedy modularity agglomeration in the
//! Clauset–Newman–Moore (2004) shape. This is an independent implementation
//! written to that integer contract, not a port of any library; networkx's
//! `greedy_modularity_communities` is prior art of the same family and shares
//! no code here.
//!
//! Contract summary, in the words the tests pin:
//! - Graph: symmetrize the directed edge list into an undirected multiset,
//!   drop self-loops. `W` = number of undirected edges.
//! - Merge predicate: exact integer `N = k_ij · 2W − K_a · K_b` (`i128`;
//!   `|N| <= (2W)^2`, exact within the caps). No floating point: float
//!   equality and integer equality disagree at the tie boundary, and ties are
//!   frequent. `dq` has the sign of `N`, so ordering by `dq` is ordering by
//!   `N`.
//! - Merge order: FULL PAIR RESCAN every merge, candidates in sorted pair
//!   order; ties on `N` break by `(min-member, min-member)` lexicographic
//!   order; stop at the first maximum with `N <= 0`. Heap/lazy recomputation
//!   changes tie winners and is prohibited (the fat-square trap in the tests
//!   shows a lazy recompute lands elsewhere on the same graph).
//! - Cluster identity: the lexicographically smallest member node id. A merge
//!   keeps the lexicographically smaller community key, and the key is the
//!   minimum member by induction, so the surviving key is the label.

use std::collections::{BTreeMap, BTreeSet};

/// The machine form's schema key (workplan §Decisions, json machine form).
pub const SCHEMA: &str = "archspec-clusters-v0";
/// Node cap from DESIGN_CONTRACTS §5; above it the overlay refuses with the
/// named skip line and computes nothing else.
pub const CAP_NODES: usize = 10_000;
/// Edge cap (round-2 correction n2): anchored on the measured PoC point
/// 79.9 s at 10 000 nodes / 40 000 edges, scaled by the 300 s heavy-leg
/// budget with the log correction — the ceiling sits near 134 000, so 130 000
/// is the rounded-down named cap. Raising it means re-deriving the anchor.
pub const CAP_EDGES: u64 = 130_000;

/// The structure labels (§4). Exactly one prints per render, evaluated in the
/// order empty → single region → trivial → `n clusters`.
pub const LABEL_EMPTY: &str = "empty";
pub const LABEL_SINGLE_REGION: &str = "none (single region)";
pub const LABEL_TRIVIAL: &str = "trivial (all singletons)";

/// The outcome of one clustering render: a named beyond-cap skip, or the
/// partition with its counts and exactly one structure label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `nodes > CAP_NODES` — checked first; nothing else computed.
    SkippedNodes {
        nodes: usize,
        edges: u64,
    },
    /// `edges > CAP_EDGES` with a within-cap node count.
    SkippedEdges {
        nodes: usize,
        edges: u64,
    },
    Partitions(Partitions),
}

/// The computed partition: counts, the §4 label, and the assignment lines'
/// data — every node with its cluster label, sorted by node id. The empty
/// class carries no assignments (the render shows no assignment lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partitions {
    pub nodes: usize,
    pub edges: u64,
    pub label: String,
    pub assignments: Vec<(String, String)>,
}

impl Outcome {
    /// The label the `structure` position carries: the §4 structure label, or
    /// the skip label the human render prints for a beyond-cap graph.
    pub fn structure_label(&self) -> String {
        match self {
            Outcome::SkippedNodes { nodes, .. } => {
                format!("skipped (nodes={nodes} > cap {CAP_NODES})")
            }
            Outcome::SkippedEdges { edges, .. } => {
                format!("skipped (edges={edges} > cap {CAP_EDGES})")
            }
            Outcome::Partitions(partitions) => partitions.label.clone(),
        }
    }

    /// The machine form's counts pair (json `nodes` / `edges`).
    pub fn counts(&self) -> (usize, u64) {
        match self {
            Outcome::SkippedNodes { nodes, edges } | Outcome::SkippedEdges { nodes, edges } => {
                (*nodes, *edges)
            }
            Outcome::Partitions(partitions) => (partitions.nodes, partitions.edges),
        }
    }

    /// The machine form's assignments: empty for the skip and empty classes.
    pub fn assignments(&self) -> &[(String, String)] {
        match self {
            Outcome::SkippedNodes { .. } | Outcome::SkippedEdges { .. } => &[],
            Outcome::Partitions(partitions) => &partitions.assignments,
        }
    }
}

/// Cluster the prepared node/edge set. `nodes` is the model's node-id set
/// (module_edges endpoints plus soft-structure members); `directed` is the
/// model's directed edge list. Determinism law: the same input always
/// produces the same bytes.
pub fn cluster(nodes: BTreeSet<String>, directed: Vec<(String, String)>) -> Outcome {
    let node_count = nodes.len();

    // Graph preparation: symmetrize into an undirected multiset, drop
    // self-loops (A_ii = 0). W counts every non-self directed edge once.
    let mut inter: BTreeMap<(String, String), u64> = BTreeMap::new();
    for (a, b) in directed {
        if a == b {
            continue;
        }
        let key = if a <= b { (a, b) } else { (b, a) };
        *inter.entry(key).or_insert(0) += 1;
    }
    let edges: u64 = inter.values().sum();

    // Caps first: beyond either cap the overlay names the skip and computes
    // nothing else. The node cap is checked first (workplan §Decisions).
    if node_count > CAP_NODES {
        return Outcome::SkippedNodes {
            nodes: node_count,
            edges,
        };
    }
    if edges > CAP_EDGES {
        return Outcome::SkippedEdges {
            nodes: node_count,
            edges,
        };
    }

    // Community state. The key is the community's lexicographically smallest
    // member: true initially, and preserved by every merge because the merge
    // keeps the smaller key (sorted pair keys) and the smaller key is the
    // smaller minimum.
    let mut members: BTreeMap<String, Vec<String>> =
        nodes.iter().map(|n| (n.clone(), vec![n.clone()])).collect();
    let mut tot: BTreeMap<String, u64> = BTreeMap::new();
    for ((a, b), k) in &inter {
        *tot.entry(a.clone()).or_insert(0) += *k;
        *tot.entry(b.clone()).or_insert(0) += *k;
    }
    let mut comm_of: BTreeMap<String, String> =
        nodes.iter().map(|n| (n.clone(), n.clone())).collect();

    // `2W` is constant across merges (a merge removes no edges), so the
    // integer predicate's scale never changes mid-run. Bound: |N| <= (2W)^2;
    // within the caps 2W <= 260 000, exact in i128 (contract §5).
    let two_w = 2i128 * edges as i128;

    while !inter.is_empty() {
        // FULL PAIR RESCAN: every live pair is re-evaluated each round.
        let mut best: Option<(i128, &str, &str, (String, String))> = None;
        for ((a, b), k) in &inter {
            let n = i128::from(*k) * two_w - i128::from(tot[a]) * i128::from(tot[b]);
            match best {
                None => best = Some((n, a, b, (a.clone(), b.clone()))),
                Some((bn, bmin_a, bmin_b, _)) => {
                    // min-member == community key (invariant above).
                    if n > bn || (n == bn && (a.as_str(), b.as_str()) < (bmin_a, bmin_b)) {
                        best = Some((n, a, b, (a.clone(), b.clone())));
                    }
                }
            }
        }
        let Some((n, _, _, (a, b))) = best else {
            break;
        };
        if n <= 0 {
            break;
        }

        // Merge b into a (sorted key pair: a < b, so a's key — the smaller
        // minimum — survives as the cluster label).
        let moved = members.remove(&b).expect("live community key");
        for node in &moved {
            comm_of.insert(node.clone(), a.clone());
        }
        members
            .get_mut(&a)
            .expect("live community key")
            .extend(moved);
        members
            .get_mut(&a)
            .expect("live community key")
            .sort_unstable();
        let merged = tot.remove(&b).expect("live community key") + tot[&a];
        tot.insert(a.clone(), merged);

        // Fold the merged community's pairs; self-pairs drop, partners
        // re-key onto `a` and their pair keys renormalize.
        let folded = std::mem::take(&mut inter);
        for ((x, y), k) in folded {
            let x = if x == b { a.clone() } else { x };
            let y = if y == b { a.clone() } else { y };
            if x == y {
                continue;
            }
            let key = if x <= y { (x, y) } else { (y, x) };
            *inter.entry(key).or_insert(0) += k;
        }
    }

    let clusters = members.len();
    let label = structure_label(clusters, node_count, edges);
    let assignments = if label == LABEL_EMPTY {
        Vec::new()
    } else {
        nodes
            .iter()
            .map(|n| {
                let cluster = comm_of[n].clone();
                (n.clone(), cluster)
            })
            .collect()
    };
    Outcome::Partitions(Partitions {
        nodes: node_count,
        edges,
        label,
        assignments,
    })
}

/// The §4 ordered labelling rule, single source: `cluster` labels every
/// state through it, and the verdict-rule test pins the four labels against
/// each other without building a graph per class.
pub fn structure_label(clusters: usize, nodes: usize, edges: u64) -> String {
    if nodes == 0 || edges == 0 {
        LABEL_EMPTY.to_string()
    } else if clusters == 1 && nodes > 1 {
        LABEL_SINGLE_REGION.to_string()
    } else if clusters == nodes {
        LABEL_TRIVIAL.to_string()
    } else {
        format!("{clusters} clusters")
    }
}

/// Build the overlay's human lines: one structure (or skip) line, then the
/// sorted assignment lines — none for the empty and skip classes.
pub fn render_human(outcome: &Outcome) -> String {
    let mut out = String::new();
    match outcome {
        Outcome::SkippedNodes { nodes, .. } => {
            out.push_str(&format!(
                "clusters: skipped (nodes={nodes} > cap {CAP_NODES})\n"
            ));
        }
        Outcome::SkippedEdges { edges, .. } => {
            out.push_str(&format!(
                "clusters: skipped (edges={edges} > cap {CAP_EDGES})\n"
            ));
        }
        Outcome::Partitions(partitions) => {
            out.push_str(&format!("structure: {}\n", partitions.label));
            for (node, cluster) in &partitions.assignments {
                out.push_str(&format!("{node} -> {cluster}\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn pairs(edges: &[(&str, &str)]) -> Vec<(String, String)> {
        edges
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn assignment_lines(outcome: &Outcome) -> Vec<String> {
        outcome
            .assignments()
            .iter()
            .map(|(n, c)| format!("{n}\t{c}"))
            .collect()
    }

    #[test]
    #[ignore = "parity scratch — run once per port with ARCHSPEC_CLUSTERS_PARITY set to a scan.json"]
    fn poc_parity_scratch_against_committed_graphs() {
        // Dev-only port verification (never shipped semantics): read a
        // scan.json, prepare the same graph, print the sha256 over the
        // sorted `node\tcluster` lines to match the PoC golden table.
        let Ok(path) = std::env::var("ARCHSPEC_CLUSTERS_PARITY") else {
            panic!("set ARCHSPEC_CLUSTERS_PARITY to a scan.json path");
        };
        let text = std::fs::read_to_string(&path).expect("scan.json");
        let value: serde_json::Value = serde_json::from_str(&text).expect("model json");
        let mut nodes = std::collections::BTreeSet::new();
        let mut directed = Vec::new();
        for e in value["module_edges"].as_array().unwrap_or(&Vec::new()) {
            let f = e["from"].as_str().expect("from").to_string();
            let t = e["to"].as_str().expect("to").to_string();
            nodes.insert(f.clone());
            nodes.insert(t.clone());
            directed.push((f, t));
        }
        for lst in value["soft_structure"]
            .as_object()
            .map(|o| o.values())
            .into_iter()
            .flatten()
        {
            for m in lst.as_array().unwrap_or(&Vec::new()) {
                if let Some(s) = m.as_str() {
                    nodes.insert(s.to_string());
                }
            }
        }
        let outcome = cluster(nodes, directed);
        let Outcome::Partitions(p) = &outcome else {
            panic!("committed graphs are within the caps")
        };
        let joined: String = p
            .assignments
            .iter()
            .map(|(n, c)| format!("{n}\t{c}\n"))
            .collect();
        eprintln!(
            "parity {} nodes={} edges={} label={}",
            path, p.nodes, p.edges, p.label
        );
        eprintln!("sha256 {}", sha256_hex(joined.as_bytes()));
    }

    fn sha256_hex(data: &[u8]) -> String {
        // Minimal SHA-256 for the dev-only parity probe above (std-only).
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let bit_len = (data.len() as u64) * 8;
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
            let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            for (acc, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
                *acc = acc.wrapping_add(v);
            }
        }
        h.iter().map(|v| format!("{v:08x}")).collect()
    }

    #[test]
    fn empty_graph_is_empty_and_names_no_assignments() {
        let outcome = cluster(BTreeSet::new(), Vec::new());
        let Outcome::Partitions(p) = &outcome else {
            panic!("within caps partitions")
        };
        assert_eq!(p.label, LABEL_EMPTY);
        assert_eq!(p.nodes, 0);
        assert_eq!(p.edges, 0);
        assert!(p.assignments.is_empty());
    }

    #[test]
    fn edgeless_nodes_and_self_loops_only_are_empty_too() {
        // Rule 1 fires on "no edges" before the trivial rule: an edgeless
        // node set and a self-loop-only graph both state `structure: empty`
        // with no assignment lines (contract §4 order).
        let edgeless = cluster(ids(&["m/a", "m/b", "m/c"]), Vec::new());
        assert_eq!(edgeless.structure_label(), LABEL_EMPTY);
        assert!(edgeless.assignments().is_empty());

        let loops = cluster(
            ids(&["m/a", "m/b"]),
            pairs(&[("m/a", "m/a"), ("m/b", "m/b")]),
        );
        assert_eq!(loops.structure_label(), LABEL_EMPTY);
        assert_eq!(loops.assignments(), &[]);
    }

    #[test]
    fn dense_k_and_star_terminate_in_one_region() {
        // The §4 reproduction facts: greedy-mod on K250 -> 1 cluster; on
        // star-2000 -> 1 cluster. Both print `structure: none (single
        // region)` and the assignment block names every node.
        let k: Vec<String> = (0..250).map(|i| format!("m/k{i:03}")).collect();
        let mut kedges = Vec::new();
        for i in 0..k.len() {
            for j in i + 1..k.len() {
                // One directed edge per pair — the undirected K250 (W =
                // 31 125), the shape the PoC measured.
                kedges.push((k[i].clone(), k[j].clone()));
            }
        }
        let outcome = cluster(k.iter().cloned().collect(), kedges);
        assert_eq!(outcome.structure_label(), LABEL_SINGLE_REGION);
        assert_eq!(outcome.assignments().len(), 250);
        for (_, cluster) in outcome.assignments() {
            assert_eq!(
                cluster, "m/k000",
                "the single region is named by its min member"
            );
        }

        let mut s: Vec<String> = vec!["m/center".to_string()];
        s.extend((0..2000).map(|i| format!("m/leaf{i:04}")));
        let sedges: Vec<(String, String)> = s[1..]
            .iter()
            .map(|leaf| ("m/center".to_string(), leaf.clone()))
            .collect();
        let star = cluster(s.iter().cloned().collect(), sedges);
        assert_eq!(star.structure_label(), LABEL_SINGLE_REGION);
        assert_eq!(star.assignments().len(), 2001);
    }

    #[test]
    fn verdict_rule_pins_the_four_labels_apart() {
        // The §4 ordered rule, label-level: exactly one label per state.
        assert_eq!(structure_label(0, 0, 0), LABEL_EMPTY);
        assert_eq!(structure_label(7, 7, 0), LABEL_EMPTY); // no edges wins
        assert_eq!(structure_label(1, 250, 31125), LABEL_SINGLE_REGION);
        assert_eq!(structure_label(1, 1, 0), LABEL_EMPTY); // nodes>1 guard
        assert_eq!(structure_label(5, 5, 3), LABEL_TRIVIAL);
        assert_eq!(structure_label(41, 110, 400), "41 clusters");
    }

    #[test]
    fn cluster_label_is_the_smallest_member() {
        // Two triangles joined by one bridge: internal merges first; the
        // bridge pair's N is negative, so two clusters survive, each named
        // by its lexicographically smallest member.
        let nodes = ids(&["m/a", "m/b", "m/c", "m/d", "m/e", "m/f"]);
        let edges = pairs(&[
            ("m/a", "m/b"),
            ("m/b", "m/c"),
            ("m/c", "m/a"),
            ("m/d", "m/e"),
            ("m/e", "m/f"),
            ("m/f", "m/d"),
            ("m/c", "m/d"),
        ]);
        let outcome = cluster(nodes, edges);
        assert_eq!(outcome.structure_label(), "2 clusters");
        let got: Vec<(String, String)> = outcome.assignments().to_vec();
        assert_eq!(
            got,
            vec![
                ("m/a".into(), "m/a".into()),
                ("m/b".into(), "m/a".into()),
                ("m/c".into(), "m/a".into()),
                ("m/d".into(), "m/d".into()),
                ("m/e".into(), "m/d".into()),
                ("m/f".into(), "m/d".into()),
            ]
        );
    }

    #[test]
    fn node_cap_skips_by_name_computing_nothing_else() {
        let nodes: BTreeSet<String> = (0..=CAP_NODES).map(|i| format!("m/n{i}")).collect();
        let outcome = cluster(nodes, Vec::new());
        let Outcome::SkippedNodes { nodes, edges } = &outcome else {
            panic!("beyond the node cap the skip names itself")
        };
        assert_eq!(*nodes, CAP_NODES + 1);
        assert_eq!(*edges, 0);
        assert_eq!(
            render_human(&outcome),
            format!(
                "clusters: skipped (nodes={} > cap {CAP_NODES})\n",
                CAP_NODES + 1
            )
        );
        // The skip replaces the structure line and every assignment.
        assert!(outcome.assignments().is_empty());
    }

    #[test]
    fn edge_cap_skips_by_name_after_the_node_cap() {
        // Within the node cap, beyond the edge cap: parallel multiplicities
        // carry W past the cap without carrying nodes.
        let nodes = ids(&["m/a", "m/b"]);
        let edges: Vec<(String, String)> = (0..=CAP_EDGES)
            .map(|_| ("m/a".to_string(), "m/b".to_string()))
            .collect();
        let outcome = cluster(nodes, edges);
        let Outcome::SkippedEdges { nodes, edges } = &outcome else {
            panic!("beyond the edge cap the skip names itself")
        };
        assert_eq!(*nodes, 2);
        assert_eq!(*edges, CAP_EDGES + 1);
        assert_eq!(
            render_human(&outcome),
            format!(
                "clusters: skipped (edges={} > cap {CAP_EDGES})\n",
                CAP_EDGES + 1
            )
        );

        // The node cap is checked FIRST: a graph beyond both caps names the
        // node skip only.
        let both_nodes: BTreeSet<String> = (0..=CAP_NODES).map(|i| format!("m/n{i}")).collect();
        let both_edges: Vec<(String, String)> = (0..=CAP_EDGES)
            .map(|_| ("m/n0".to_string(), "m/n1".to_string()))
            .collect();
        let outcome = cluster(both_nodes, both_edges);
        assert!(matches!(outcome, Outcome::SkippedNodes { .. }));
    }

    #[test]
    fn full_rescan_fat_square_trap_pins_the_full_scan_answer() {
        // The lazy-recompute trap (contract §5, round-1 finding): the "fat
        // square" — four corner K2 pairs joined by doubled ring edges — has
        // full-rescan answer FOUR clusters; a heap/lazy recompute diverges
        // (it merges the ring pairs later and lands on one or two clusters;
        // proven live against the PoC semantics on this shape). A port that
        // silently switches to lazy recomputation flips this golden.
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for corner in 0..4 {
            let a = format!("c{corner}a");
            let b = format!("c{corner}b");
            nodes.push(a.clone());
            nodes.push(b.clone());
            edges.push((a.clone(), b.clone()));
            edges.push((b.clone(), a.clone())); // doubled corner edge
            let next = format!("c{}a", (corner + 1) % 4);
            edges.push((b.clone(), next.clone()));
            edges.push((next.clone(), b.clone())); // doubled ring edge
        }
        let outcome = cluster(nodes.iter().cloned().collect(), edges);
        assert_eq!(outcome.structure_label(), "4 clusters");
        let lines = assignment_lines(&outcome);
        assert_eq!(
            lines,
            vec![
                "c0a\tc0a", "c0b\tc0a", "c1a\tc1a", "c1b\tc1a", "c2a\tc2a", "c2b\tc2a", "c3a\tc3a",
                "c3b\tc3a",
            ]
        );
    }

    #[test]
    fn tie_dense_graph_is_order_stable() {
        // Equal-predicate pairs (the round-1 tie class): three disjoint
        // edges — every pair has the same N; the sorted-pair scan plus the
        // (min-member, min-member) rule fixes the merge order and the
        // result: each pair becomes one cluster.
        let nodes = ids(&["t/a", "t/b", "t/c", "t/d", "t/e", "t/f"]);
        let edges = pairs(&[("t/a", "t/b"), ("t/c", "t/d"), ("t/e", "t/f")]);
        let outcome = cluster(nodes, edges);
        assert_eq!(outcome.structure_label(), "3 clusters");
        let lines = assignment_lines(&outcome);
        assert_eq!(
            lines,
            vec![
                "t/a\tt/a", "t/b\tt/a", "t/c\tt/c", "t/d\tt/c", "t/e\tt/e", "t/f\tt/e",
            ]
        );
    }

    #[test]
    fn multiedge_weights_merge_before_the_same_signed_single_edge() {
        // Integer-exactness witness: with a doubled edge the pair predicate
        // is strictly greater than the single-edge pair's, so it merges
        // first — a float dQ port would sit on the same rational and could
        // flip the order.
        let nodes = ids(&["p/a", "p/b", "p/c"]);
        let edges = pairs(&[("p/a", "p/b"), ("p/b", "p/a"), ("p/b", "p/c")]);
        let outcome = cluster(nodes, edges);
        // a<->b (weight 2) merges first; the survivor-vs-c pair then has
        // N = 1*2W - Ka*Kb with W=3, Ka=3, Kc=1: N = 6-3 = 3 > 0, so all
        // three land in one region named by p/a.
        assert_eq!(outcome.structure_label(), LABEL_SINGLE_REGION);
        for (_, cluster) in outcome.assignments() {
            assert_eq!(cluster, "p/a");
        }
    }
}
