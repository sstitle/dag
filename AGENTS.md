# Project Guidelines

## Environment

- This repo is managed by a Nix flake in `flake.nix`, which provides a devShell and
  formatter. See https://wiki.nixos.org/wiki/Flakes
- Run `direnv allow` once to automatically load the dev environment when entering the
  directory.
- This repo uses `mask` with `maskfile.md` as the task runner. Run `mask --help` to see
  available tasks. See https://github.com/jacobdeichert/mask

## Testing

All development follows **red-green** test-driven development:

1. **Red** — write a failing test that specifies the desired behavior before writing any
   implementation.
1. **Green** — write the minimal implementation to make the test pass.
1. **Refactor** — clean up the implementation while keeping tests green.

Never write implementation code without a corresponding failing test first, unless the
code is not meaningfully testable (e.g. top-level glue/wiring, configuration, or
one-off scripts). In those cases, note why tests were skipped.

Use the unit testing framework appropriate for the language.

- **Python** — `pytest`

Run tests with:

```
mask test
```

Formatting for the whole repository (Nix, Markdown, Rust, and more) is enforced by **treefmt** via `nix flake check` or `mask format` (`nix fmt`). The Rust CI job also runs `cargo fmt`; if they disagree, **treefmt wins** — run `mask format` before pushing.
