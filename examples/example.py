"""
Pipeline DAG example using the dag Python binding.

Graph layout:
    ingest → validate → transform → aggregate → report
                              ↑
                         (also from ingest)
"""

from dag import Dag

def main():
    g = Dag()

    # Add five pipeline nodes with metadata dicts.
    ingest    = g.add_node({"name": "ingest",    "owner": "data-eng", "sla_minutes": 5})
    validate  = g.add_node({"name": "validate",  "owner": "data-eng", "sla_minutes": 2})
    transform = g.add_node({"name": "transform", "owner": "ml-team",  "sla_minutes": 10})
    aggregate = g.add_node({"name": "aggregate", "owner": "ml-team",  "sla_minutes": 4})
    report    = g.add_node({"name": "report",    "owner": "analytics","sla_minutes": 1})

    # Wire up the pipeline.
    g.add_edge(ingest,    validate,  {"type": "raw_data"})
    g.add_edge(validate,  transform, {"type": "clean_data"})
    g.add_edge(ingest,    transform, {"type": "raw_data"})   # second path to transform
    g.add_edge(transform, aggregate, {"type": "features"})
    g.add_edge(aggregate, report,    {"type": "metrics"})

    # Demonstrate cycle rejection.
    try:
        g.add_edge(report, ingest, {"type": "CYCLE"})
        print("ERROR: cycle was not rejected!")
    except ValueError as exc:
        print(f"Cycle correctly rejected: {exc}")

    # Attach extra metadata to transform.
    g.set_node_meta(transform, {**g.node_meta(transform), "gpu_required": True})

    # Query descendants of ingest.
    desc_ids = g.descendants(ingest)
    desc_names = [g.node_meta(n)["name"] for n in desc_ids]
    print(f"\nDescendants of 'ingest': {sorted(desc_names)}")

    # Query ancestors of report.
    anc_ids = g.ancestors(report)
    anc_names = [g.node_meta(n)["name"] for n in anc_ids]
    print(f"Ancestors of 'report':   {sorted(anc_names)}")

    # Topological sort — safe execution order for the pipeline.
    topo = g.topological_sort()
    topo_names = [g.node_meta(n)["name"] for n in topo]
    print(f"\nTopological order: {topo_names}")

    # Path queries.
    print(f"\nhas_path(ingest → report):  {g.has_path(ingest, report)}")
    print(f"has_path(report → ingest):  {g.has_path(report, ingest)}")

    # Roots and leaves.
    root_names  = [g.node_meta(n)["name"] for n in g.roots()]
    leaf_names  = [g.node_meta(n)["name"] for n in g.leaves()]
    print(f"\nRoots:  {root_names}")
    print(f"Leaves: {leaf_names}")

if __name__ == "__main__":
    main()
