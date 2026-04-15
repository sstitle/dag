# Maskfile

This project uses [mask](https://github.com/jacobdeichert/mask) as a task runner.
Run tasks from the repo root (e.g. inside `nix develop`).

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

> Rust (`dag-core` + `serde`), Python (pytest), Node (`npm test`), and `nix-unit` on `test.nix`

```bash
set -e
cargo test -p dag-core
cargo test -p dag-core --features serde
rm -rf .venv
python3 -m venv .venv
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path bindings/python/Cargo.toml
.venv/bin/pip install pytest hypothesis -q
.venv/bin/pytest bindings/python/tests/ -v
( cd bindings/node && npm install && npm run build && npm test )
nix-unit ./test.nix
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
