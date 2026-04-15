/**
 * Pipeline DAG example using the dag Node.js binding.
 *
 * Graph layout:
 *   ingest → validate → transform → aggregate → report
 *                            ↑
 *                      (also from ingest)
 *
 * Run after `make build-node`:
 *   cd bindings/node && npm run example
 */

import { Dag } from "../bindings/node";

interface NodeMeta {
  name: string;
  owner: string;
  sla_minutes: number;
  gpu_required?: boolean;
}

interface EdgeMeta {
  type: string;
}

function main(): void {
  const g = new Dag();

  // Add five pipeline nodes with metadata objects.
  const ingest    = g.addNode({ name: "ingest",    owner: "data-eng",  sla_minutes: 5  } satisfies NodeMeta);
  const validate  = g.addNode({ name: "validate",  owner: "data-eng",  sla_minutes: 2  } satisfies NodeMeta);
  const transform = g.addNode({ name: "transform", owner: "ml-team",   sla_minutes: 10 } satisfies NodeMeta);
  const aggregate = g.addNode({ name: "aggregate", owner: "ml-team",   sla_minutes: 4  } satisfies NodeMeta);
  const report    = g.addNode({ name: "report",    owner: "analytics", sla_minutes: 1  } satisfies NodeMeta);

  // Wire up the pipeline.
  g.addEdge(ingest,    validate,  { type: "raw_data"   } satisfies EdgeMeta);
  g.addEdge(validate,  transform, { type: "clean_data" } satisfies EdgeMeta);
  g.addEdge(ingest,    transform, { type: "raw_data"   } satisfies EdgeMeta);  // second path
  g.addEdge(transform, aggregate, { type: "features"   } satisfies EdgeMeta);
  g.addEdge(aggregate, report,    { type: "metrics"    } satisfies EdgeMeta);

  // Demonstrate cycle rejection.
  try {
    g.addEdge(report, ingest, { type: "CYCLE" });
    console.error("ERROR: cycle was not rejected!");
  } catch (err) {
    console.log(`Cycle correctly rejected: ${(err as Error).message}`);
  }

  // Attach an extra field to transform's metadata.
  const transformMeta = g.nodeMeta(transform) as NodeMeta;
  g.setNodeMeta(transform, { ...transformMeta, gpu_required: true });

  // Query descendants of ingest.
  const descIds   = g.descendants(ingest);
  const descNames = descIds.map((id) => (g.nodeMeta(id) as NodeMeta).name).sort();
  console.log(`\nDescendants of 'ingest': [${descNames.join(", ")}]`);

  // Query ancestors of report.
  const ancIds   = g.ancestors(report);
  const ancNames = ancIds.map((id) => (g.nodeMeta(id) as NodeMeta).name).sort();
  console.log(`Ancestors of 'report':   [${ancNames.join(", ")}]`);

  // Topological sort — safe execution order for the pipeline.
  const topo      = g.topologicalSort();
  const topoNames = topo.map((id) => (g.nodeMeta(id) as NodeMeta).name);
  console.log(`\nTopological order: [${topoNames.join(", ")}]`);

  // Path queries.
  console.log(`\nhas_path(ingest → report):  ${g.hasPath(ingest, report)}`);
  console.log(`has_path(report → ingest):  ${g.hasPath(report, ingest)}`);

  // Roots and leaves.
  const rootNames = g.roots().map((id) => (g.nodeMeta(id) as NodeMeta).name);
  const leafNames = g.leaves().map((id) => (g.nodeMeta(id) as NodeMeta).name);
  console.log(`\nRoots:  [${rootNames.join(", ")}]`);
  console.log(`Leaves: [${leafNames.join(", ")}]`);
}

main();
