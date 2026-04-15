# Maskfile

This project uses [mask](https://github.com/jacobdeichert/mask) as a task runner.
Run tasks from the repo root. Prefer **`nix develop`** (or `nix develop -c mask <task>`) so
Python, Rust, maturin, and Node match the flake and CI; ad-hoc shells can hit toolchain
mismatches (e.g. maturin targeting the wrong host).

## format

> Format tracked files with treefmt (`nix fmt`)

```bash
nix fmt
```

## build

> Build Python (fresh `.venv` + maturin) and Node (`npm install` + `npm run build`) bindings

```bash
set -e
rm -rf .venv
python3 -m venv .venv
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path bindings/python/Cargo.toml
( cd bindings/node && npm install && npm run build )
```

## test

> Rust (`dag-core` with `--all-features`, including `serde` and `raw-id-access`), Python (pytest + hypothesis), and Node (`npm test`)

```bash
set -e
cargo test -p dag-core --all-features
rm -rf .venv
python3 -m venv .venv
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path bindings/python/Cargo.toml
.venv/bin/pip install pytest hypothesis -q
.venv/bin/pytest bindings/python/tests/ -v
( cd bindings/node && npm install && npm run build && npm test )
```

## run

> Run `examples/example.py` and the Node `example` script (creates `.venv` + builds Python binding if missing)

```bash
set -e
if [ ! -f .venv/bin/python ]; then
  rm -rf .venv
  python3 -m venv .venv
  VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path bindings/python/Cargo.toml
fi
.venv/bin/python examples/example.py
( cd bindings/node && npm install && npm run example )
```
