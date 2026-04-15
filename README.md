# dag

A directed acyclic graph (DAG) library written in Rust with Python and Node.js bindings.
Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Features

- Generic `Dag<N, E>` parameterized over node metadata `N` and edge metadata `E`
- Cycle detection on every edge insertion
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
- **`DagError`** — `NodeNotFound`, `EdgeNotFound`, `CycleDetected`
- **`Dag<N, E>`** — the graph itself; not thread-safe without external locking

### Rust

```rust
use dag_core::{Dag, DagError};

let mut dag: Dag<&str, ()> = Dag::new();
let n1 = dag.add_node("fetch");
let n2 = dag.add_node("transform");
let n3 = dag.add_node("load");

let e = dag.add_edge(n1, n2, ())?;  // Err(DagError::CycleDetected) if cycle
dag.add_edge(n2, n3, ())?;

let order = dag.topological_sort();  // [n1, n2, n3]
let (from, to) = dag.edge_endpoints(e)?;

dag.remove_edge(e)?;   // remove edge, keep nodes
dag.remove_node(n1)?;  // remove node and all incident edges
```

`ancestors()` and `descendants()` return unordered `Vec<NodeId>`.
`topological_sort()` breaks ties deterministically by `NodeId` value.

#### Serde

Enable the `serde` feature to derive `Serialize`/`Deserialize` for `Dag<N, E>`:

```toml
dag-core = { version = "0.1", features = ["serde"] }
```

### Python

```python
from dag import Dag, DagNodeNotFoundError, DagEdgeNotFoundError, DagCycleError

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
```

Install: `pip install dag` (after building with `maturin`)

Run tests: `mask test-python`

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
```

Install: `npm install @dag-rs/dag`

Run tests: `mask test-node`

## Development

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled, or the tools listed in `flake.nix` installed manually
- [direnv](https://direnv.net/) for automatic environment loading (optional)

### Quick start

```bash
direnv allow        # or: nix develop
mask setup          # build Python and Node bindings
mask test-all       # run all test suites
```

### Available tasks

```
mask test           # Rust core (+ serde feature)
mask test-python    # build Python binding, run pytest
mask test-node      # build Node binding, run node:test
mask test-all       # all three
mask build-python   # build Python extension into .venv
mask build-node     # build Node native extension
mask run-examples   # run examples/example.py and examples/example.ts
```

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
.github/workflows/      CI (Rust + Python + Node on every push/PR)
```
