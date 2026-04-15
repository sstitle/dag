use crate::{Dag, DagError};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build a simple chain: n1 → n2 → n3.
fn chain() -> (Dag<&'static str, ()>, [crate::NodeId; 3]) {
    let mut dag = Dag::new();
    let n1 = dag.add_node("a");
    let n2 = dag.add_node("b");
    let n3 = dag.add_node("c");
    dag.add_edge(n1, n2, ()).unwrap();
    dag.add_edge(n2, n3, ()).unwrap();
    (dag, [n1, n2, n3])
}

// ── basic construction ────────────────────────────────────────────────────────

#[test]
fn test_add_node_and_meta() {
    let mut dag: Dag<i32, i32> = Dag::new();
    let n1 = dag.add_node(42);
    let n2 = dag.add_node(99);

    assert_eq!(dag.node_meta(n1).unwrap(), &42);
    assert_eq!(dag.node_meta(n2).unwrap(), &99);
}

#[test]
fn test_add_edge_and_meta() {
    let mut dag: Dag<(), i32> = Dag::new();
    let n1 = dag.add_node(());
    let n2 = dag.add_node(());
    let e = dag.add_edge(n1, n2, 7).unwrap();

    assert_eq!(dag.edge_meta(e).unwrap(), &7);
}

#[test]
fn test_set_node_meta() {
    let mut dag: Dag<i32, ()> = Dag::new();
    let n = dag.add_node(1);
    dag.set_node_meta(n, 2).unwrap();
    assert_eq!(dag.node_meta(n).unwrap(), &2);
}

#[test]
fn test_set_edge_meta() {
    let mut dag: Dag<(), i32> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let e = dag.add_edge(a, b, 0).unwrap();
    dag.set_edge_meta(e, 99).unwrap();
    assert_eq!(dag.edge_meta(e).unwrap(), &99);
}

#[test]
fn test_node_not_found_error() {
    let (dag, [n1, n2, n3]) = chain();
    let _ = (n1, n2, n3); // suppress unused warning
    // Create a dangling id by removing a node.
    let mut dag2: Dag<(), ()> = Dag::new();
    let x = dag2.add_node(());
    dag2.remove_node(x).unwrap();
    assert!(matches!(dag2.node_meta(x), Err(DagError::NodeNotFound(_))));
    let _ = dag;
}

// ── cycle rejection ────────────────────────────────────────────────────────────

#[test]
fn test_self_loop_rejected() {
    let mut dag: Dag<(), ()> = Dag::new();
    let n = dag.add_node(());
    assert!(matches!(dag.add_edge(n, n, ()), Err(DagError::CycleDetected)));
}

#[test]
fn test_direct_back_edge_rejected() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    assert!(matches!(dag.add_edge(b, a, ()), Err(DagError::CycleDetected)));
}

#[test]
fn test_transitive_cycle_rejected() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let c = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    dag.add_edge(b, c, ()).unwrap();
    // c → a would close the cycle a → b → c → a.
    assert!(matches!(dag.add_edge(c, a, ()), Err(DagError::CycleDetected)));
}

#[test]
fn test_parallel_edges_allowed() {
    // Two separate roots both pointing to the same child is fine.
    let mut dag: Dag<(), ()> = Dag::new();
    let r1 = dag.add_node(());
    let r2 = dag.add_node(());
    let child = dag.add_node(());
    dag.add_edge(r1, child, ()).unwrap();
    dag.add_edge(r2, child, ()).unwrap();
}

#[test]
fn test_diamond_no_cycle() {
    // a → b, a → c, b → d, c → d  — diamond, not a cycle.
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let c = dag.add_node(());
    let d = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    dag.add_edge(a, c, ()).unwrap();
    dag.add_edge(b, d, ()).unwrap();
    dag.add_edge(c, d, ()).unwrap();
}

// ── ancestors / descendants ───────────────────────────────────────────────────

#[test]
fn test_ancestors_chain() {
    let (dag, [n1, n2, n3]) = chain();

    let anc = dag.ancestors(n3).unwrap();
    assert_eq!(anc.len(), 2);
    assert!(anc.contains(&n1));
    assert!(anc.contains(&n2));

    assert!(dag.ancestors(n1).unwrap().is_empty());
}

#[test]
fn test_ancestors_diamond() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let c = dag.add_node(());
    let d = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    dag.add_edge(a, c, ()).unwrap();
    dag.add_edge(b, d, ()).unwrap();
    dag.add_edge(c, d, ()).unwrap();

    let anc = dag.ancestors(d).unwrap();
    assert_eq!(anc.len(), 3);
    assert!(anc.contains(&a));
    assert!(anc.contains(&b));
    assert!(anc.contains(&c));
}

#[test]
fn test_descendants_chain() {
    let (dag, [n1, n2, n3]) = chain();

    let desc = dag.descendants(n1).unwrap();
    assert_eq!(desc.len(), 2);
    assert!(desc.contains(&n2));
    assert!(desc.contains(&n3));

    assert!(dag.descendants(n3).unwrap().is_empty());
}

#[test]
fn test_descendants_diamond() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let c = dag.add_node(());
    let d = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    dag.add_edge(a, c, ()).unwrap();
    dag.add_edge(b, d, ()).unwrap();
    dag.add_edge(c, d, ()).unwrap();

    let desc = dag.descendants(a).unwrap();
    assert_eq!(desc.len(), 3);
    assert!(desc.contains(&b));
    assert!(desc.contains(&c));
    assert!(desc.contains(&d));
}

#[test]
fn test_disconnected_node_no_ancestors_no_descendants() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let _b = dag.add_node(());
    assert!(dag.ancestors(a).unwrap().is_empty());
    assert!(dag.descendants(a).unwrap().is_empty());
}

// ── topological sort ──────────────────────────────────────────────────────────

fn topo_valid<N, E>(dag: &Dag<N, E>, order: &[crate::NodeId]) -> bool {
    use std::collections::HashMap;
    let pos: HashMap<crate::NodeId, usize> =
        order.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    // Every node must appear exactly once — verify count matches order length.
    if pos.len() != order.len() {
        return false;
    }
    // For every edge u→v, pos[u] < pos[v].
    dag.nodes
        .iter()
        .all(|(k, node)| {
            let u = crate::NodeId::from(k);
            node.out_edges.iter().all(|eid| {
                let v = dag.edges[eid.key()].to;
                pos[&u] < pos[&v]
            })
        })
}

#[test]
fn test_topo_sort_chain() {
    let (dag, [n1, n2, n3]) = chain();
    let order = dag.topological_sort();
    assert_eq!(order.len(), 3);
    assert!(topo_valid(&dag, &order));
    // In a chain the order must be exactly n1, n2, n3.
    assert_eq!(order, vec![n1, n2, n3]);
}

#[test]
fn test_topo_sort_diamond() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    let c = dag.add_node(());
    let d = dag.add_node(());
    dag.add_edge(a, b, ()).unwrap();
    dag.add_edge(a, c, ()).unwrap();
    dag.add_edge(b, d, ()).unwrap();
    dag.add_edge(c, d, ()).unwrap();

    let order = dag.topological_sort();
    assert_eq!(order.len(), 4);
    assert!(topo_valid(&dag, &order));
}

#[test]
fn test_topo_sort_empty() {
    let dag: Dag<(), ()> = Dag::new();
    assert!(dag.topological_sort().is_empty());
}

#[test]
fn test_topo_sort_single_node() {
    let mut dag: Dag<(), ()> = Dag::new();
    let n = dag.add_node(());
    assert_eq!(dag.topological_sort(), vec![n]);
}

// ── roots / leaves ────────────────────────────────────────────────────────────

#[test]
fn test_roots_and_leaves_chain() {
    let (dag, [n1, _, n3]) = chain();

    let mut roots = dag.roots();
    roots.sort();
    assert_eq!(roots, vec![n1]);

    let mut leaves = dag.leaves();
    leaves.sort();
    assert_eq!(leaves, vec![n3]);
}

#[test]
fn test_roots_multiple() {
    let mut dag: Dag<(), ()> = Dag::new();
    let r1 = dag.add_node(());
    let r2 = dag.add_node(());
    let child = dag.add_node(());
    dag.add_edge(r1, child, ()).unwrap();
    dag.add_edge(r2, child, ()).unwrap();

    let mut roots = dag.roots();
    roots.sort();
    assert!(roots.contains(&r1));
    assert!(roots.contains(&r2));
    assert_eq!(roots.len(), 2);

    let leaves = dag.leaves();
    assert_eq!(leaves, vec![child]);
}

#[test]
fn test_isolated_node_is_both_root_and_leaf() {
    let mut dag: Dag<(), ()> = Dag::new();
    let n = dag.add_node(());
    assert!(dag.roots().contains(&n));
    assert!(dag.leaves().contains(&n));
}

// ── has_path ──────────────────────────────────────────────────────────────────

#[test]
fn test_has_path_direct() {
    let (dag, [n1, n2, _]) = chain();
    assert!(dag.has_path(n1, n2).unwrap());
}

#[test]
fn test_has_path_transitive() {
    let (dag, [n1, _, n3]) = chain();
    assert!(dag.has_path(n1, n3).unwrap());
}

#[test]
fn test_has_path_reverse_false() {
    let (dag, [n1, _, n3]) = chain();
    assert!(!dag.has_path(n3, n1).unwrap());
}

#[test]
fn test_has_path_same_node() {
    let (dag, [n1, _, _]) = chain();
    assert!(dag.has_path(n1, n1).unwrap());
}

#[test]
fn test_has_path_disconnected() {
    let mut dag: Dag<(), ()> = Dag::new();
    let a = dag.add_node(());
    let b = dag.add_node(());
    assert!(!dag.has_path(a, b).unwrap());
}

// ── remove_node ───────────────────────────────────────────────────────────────

#[test]
fn test_remove_middle_node() {
    let (mut dag, [n1, n2, n3]) = chain();
    dag.remove_node(n2).unwrap();

    assert!(dag.node_meta(n2).is_err());
    assert!(dag.node_meta(n1).is_ok());
    assert!(dag.node_meta(n3).is_ok());

    // After removing n2, n1 and n3 are disconnected.
    assert!(!dag.has_path(n1, n3).unwrap());
    // n1 should now be both a root and a leaf.
    assert!(dag.roots().contains(&n1));
    assert!(dag.leaves().contains(&n1));
}

#[test]
fn test_remove_nonexistent_node_errors() {
    let (mut dag, [_, n2, _]) = chain();
    dag.remove_node(n2).unwrap();
    // Removing again is an error.
    assert!(dag.remove_node(n2).is_err());
}
