# dag

A directed acyclic graph (DAG) library written in Rust with Python and Node.js bindings.
Licensed under [MIT](LICENSE).

## Features

- Generic `Dag<N, E, P>` parameterized over node metadata `N`, edge metadata `E`, and optional
  cycle-check policy `P` (defaults to checking every `add_edge`; use `SkipCycleCheck` only
  when you bulk-load a pre-validated acyclic graph)
- Cycle detection on every edge insertion (with the default policy)
- Full graph query API: ancestors, descendants, roots, leaves, topological sort, path queries
- Edge-level mutation: add, remove, and update individual edges without cascading deletion
- Node and edge introspection: iterate all IDs, query endpoints
- JSON serialization with round-trip ID preservation (optional `serde` feature)
- Python bindings with typed exceptions and type stubs
- Node.js bindings with TypeScript declarations
- Multi-platform native extension support

## API

### Core concepts

- **`NodeId`** / **`EdgeId`** — opaque numeric identifiers returned by insertion methods
- **`DagError`** — `NodeNotFound`, `EdgeNotFound`, `CycleDetected` (bad edge insert), `NotAcyclic` (graph has a cycle), `DuplicateEdge`
- **`Dag<N, E, P>`** — the graph itself (`P` is the [`CyclePolicy`](core/src/lib.rs) type
  parameter); not thread-safe without external locking

#### Performance (Rust)

- With the default cycle policy, each `add_edge` runs a reachability scan (**O(V + E)** in the
  worst case) plus a duplicate-edge check (**O(out-degree(from))**). For bulk-loading edges you
  already know are acyclic, use `Dag<N, E, SkipCycleCheck>` and only call
  [`topological_sort`](core/src/lib.rs) (or another validation pass) once at the end.
- Prefer [`iter_nodes`](core/src/lib.rs) / [`iter_edges`](core/src/lib.rs) when you only need to
  scan IDs; [`nodes`](core/src/lib.rs) / [`edges`](core/src/lib.rs) collect into a new \[`Vec`\]
  each time.

### Rust

```rust
use dag_core::{Dag, DagError};

let mut dag: Dag<&str, ()> = Dag::new();
let n1 = dag.add_node("fetch");
let n2 = dag.add_node("transform");
let n3 = dag.add_node("load");

let e = dag.add_edge(n1, n2, ())?;  // Err(DagError::CycleDetected) if cycle
dag.add_edge(n2, n3, ())?;

let order = dag.topological_sort()?; // [n1, n2, n3]; Err(DagError::NotAcyclic) if cyclic
let (from, to) = dag.edge_endpoints(e)?;

dag.remove_edge(e)?;   // remove edge, keep nodes
dag.remove_node(n1)?;  // remove node and all incident edges
```

`ancestors()` returns every **upstream** (transitive predecessor) node — vertices `u` with a path `u → … → target`. `descendants()` returns every **downstream** (transitive successor) node. Both return unordered `Vec<NodeId>` and allocate; order is also non-deterministic across processes due to hashing.
`topological_sort()` breaks ties deterministically by `NodeId` value and returns
`Err(DagError::NotAcyclic)` when the graph contains a cycle. It runs the same **O(V + E)**
Kahn traversal as `validate_acyclic()`; if you need both a pass/fail check and the order,
call `topological_sort()` once instead of calling both.

For FFI, enable the `raw-id-access` feature on `dag-core` if you need `NodeId::from_raw` /
`EdgeId::from_raw` (used by the Node.js binding).

#### Serde

Enable the `serde` feature to derive `Serialize`/`Deserialize` for `Dag<N, E, P>`:

```toml
dag-core = { version = "0.1", features = ["serde"] }
```

Use [`parse_dag_from_json_str`](core/src/lib.rs) with [`DEFAULT_MAX_DAG_JSON_BYTES`](core/src/lib.rs)
to deserialise from a string while rejecting oversized input before parsing (important for
untrusted JSON):

```rust
use dag_core::{parse_dag_from_json_str, Dag, DEFAULT_MAX_DAG_JSON_BYTES};

let dag: Dag<String, ()> =
    parse_dag_from_json_str(json_str, DEFAULT_MAX_DAG_JSON_BYTES)?;
dag.validate_acyclic()?; // JSON does not prove acyclicity; check untrusted payloads
```

### Python

```python
from dag import Dag, DagNodeNotFoundError, DagEdgeNotFoundError, DagCycleError, DagNotAcyclicError

dag = Dag()
n1 = dag.add_node("fetch")
n2 = dag.add_node("transform")
e  = dag.add_edge(n1, n2, {"weight": 1.0})

try:
    dag.add_edge(n2, n1, None)   # raises DagCycleError
except DagCycleError:
    pass

from_node, to_node = dag.edge_endpoints(e)
dag.remove_edge(e)

# Serialisation
json_str = dag.to_json()
dag2 = Dag.from_json(json_str)  # IDs are preserved
dag2.validate_acyclic()  # optional: JSON does not prove acyclicity

order = dag.topological_sort()  # raises DagNotAcyclicError if the graph is not acyclic
```

Integer metadata is limited to the range representable in JSON as a 64-bit signed or unsigned
integer (roughly `i64` / `u64`); larger Python integers raise `ValueError`.

`Dag.from_json` rejects strings longer than `DEFAULT_MAX_DAG_JSON_BYTES` (256 MiB) before
parsing; pass `max_bytes=` to override (for example in tests). Deserialisation does not
prove the graph is acyclic; for untrusted JSON, call `validate_acyclic()` (or
`topological_sort()`) after loading. Metadata conversion is limited to
`MAX_JSON_CONVERSION_DEPTH` levels of nested `list`/`dict` to avoid stack exhaustion.

Install: `pip install dag` (after building with `maturin`)

Run tests: `mask test`

### Node.js

```js
const { Dag } = require('@dag-rs/dag');

const dag = new Dag();
const n1 = dag.addNode('fetch');
const n2 = dag.addNode('transform');
const e  = dag.addEdge(n1, n2, { weight: 1.0 });

const [from, to] = dag.edgeEndpoints(e);
dag.removeEdge(e);

const json = dag.toJson();
const dag2 = Dag.fromJson(json);  // IDs are preserved
dag2.validateAcyclic(); // JSON does not prove acyclicity; check untrusted payloads
```

Install: `npm install @dag-rs/dag`

Errors thrown by the native binding use stable message prefixes (`DAG_NODE_NOT_FOUND:`,
`DAG_EDGE_NOT_FOUND:`, `DAG_CYCLE_DETECTED:` (adding an edge would create a cycle),
`DAG_NOT_ACYCLIC:` (the graph already contains a cycle — failed `topologicalSort` or `validateAcyclic`),
`DAG_DUPLICATE_EDGE:`, `DAG_INVALID_ID:`, `DAG_ID_NOT_REPRESENTABLE:`,
`DAG_JSON_TOO_LARGE:`, `DAG_JSON_PARSE:`) so you can branch without `instanceof`. The package
also exports string constants (`DAG_ERROR_CODE_NODE_NOT_FOUND`, `DAG_ERROR_CODE_JSON_TOO_LARGE`,
and others) for comparisons. `Dag.fromJson` rejects strings longer than
`defaultMaxDagJsonBytes()` before parsing; pass an optional second argument to override (for
example in tests). Node and edge IDs must be **non-negative integers** within JavaScript’s
safe integer range (`Number.MIN_SAFE_INTEGER` … `Number.MAX_SAFE_INTEGER`) when passed into
methods. Methods that return node or edge IDs throw `DAG_ID_NOT_REPRESENTABLE` if an ID cannot
be represented as a JavaScript safe integer.

## Development

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled, or the tools listed in `flake.nix` installed manually
- [direnv](https://direnv.net/) for automatic environment loading (optional)

### Quick start

```bash
direnv allow        # or: nix develop
mask build          # Python + Node bindings
mask test           # Rust, Python, and Node
```

### Available tasks

```
mask format   # treefmt (nix fmt) — authoritative for Nix, Markdown, Rust, …
mask build    # Python .venv + maturin, Node npm build
mask test     # cargo, pytest, npm test
mask run      # example scripts (Python + Node)
```

`nix flake check` (run in CI) verifies treefmt formatting via the flake `checks` output. The
Rust CI job also runs `cargo fmt`; if anything disagrees, **treefmt** (`mask format`) is the
source of truth for the repo.

## Project structure

```
core/                   Rust core library (dag-core)
bindings/
  python/               PyO3 / maturin Python extension
    tests/              pytest test suite
    dag.pyi             Python type stubs
  node/                 napi-rs Node.js extension
    test/               node:test test suite
    index.d.ts          TypeScript declarations
examples/               Runnable Python and TypeScript examples
.github/workflows/      CI (Rust + Python + Node + Nix flake check on every push/PR)
```
