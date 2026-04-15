# Maskfile

This project uses [mask](https://github.com/jacobdeichert/mask) as a task runner.
All tasks below are available via `mask <task>` inside `nix develop`.

## test

> Run the Rust core test suite (including serde feature)

```bash
cargo test -p dag-core
cargo test -p dag-core --features serde
```

## test-python

> Build the Python binding and run pytest

```bash
mask build-python
.venv/bin/pip install pytest -q
.venv/bin/pytest bindings/python/tests/ -v
```

## test-node

> Build the Node binding and run the test suite

```bash
mask build-node
cd bindings/node && npm test
```

## test-all

> Run all test suites (Rust, Python, Node)

```bash
mask test
mask test-python
mask test-node
```

## test-nix

> Run Nix unit tests

```bash
nix-unit ./test.nix
```

## build-python

> Build the Python binding and install it into a fresh .venv (always recreated to avoid arch mismatches)

```bash
rm -rf .venv
python3 -m venv .venv
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path bindings/python/Cargo.toml
```

## build-node

> Build the Node.js binding for the current platform — produces index.<platform>.node + index.js + index.d.ts

```bash
cd bindings/node && npm install && npm run build
```

## setup

> Build both bindings from scratch (run this once after cloning or switching machines)

```bash
mask build-python
mask build-node
```

## run-examples

> Run both example scripts (builds Python binding first if .venv is absent)

```bash
if [ ! -f .venv/bin/python ]; then
  mask build-python
fi
.venv/bin/python examples/example.py
cd bindings/node && npm run example
```
