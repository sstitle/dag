# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Dag::remove_edge(id)` — remove a single edge without touching its endpoint nodes
- `Dag::nodes()` — return all node IDs in the graph
- `Dag::edges()` — return all edge IDs in the graph
- `Dag::edge_endpoints(id)` — return the `(from, to)` node pair for an edge
- Optional `serde` feature on `dag-core` for `Serialize`/`Deserialize` support
- `Dag.to_json()` / `Dag.from_json()` serialization in Python and Node bindings
- Distinct Python exception types: `DagNodeNotFoundError`, `DagEdgeNotFoundError`, `DagCycleError`
- Python type stubs (`dag.pyi`) for IDE support and static analysis
- Test suites for Python (pytest, 37 tests) and Node (node:test, 31 tests)
- GitHub Actions CI running Rust, Python, and Node tests on every push/PR
- `LICENSE-MIT` and `LICENSE-APACHE` (dual-licensed MIT OR Apache-2.0)
- Node package renamed to `@dag-rs/dag` to avoid npm namespace conflicts

## [0.1.0] — 2024

### Added

- Initial `Dag<N, E>` core library with generic node and edge metadata
- `add_node`, `remove_node`, `add_edge` mutation API
- `ancestors`, `descendants`, `roots`, `leaves`, `topological_sort`, `has_path` queries
- Cycle detection via reachability check before edge insertion
- Python bindings via PyO3 / maturin
- Node.js bindings via napi-rs
- Nix flake development environment
- 29 unit tests for the Rust core
