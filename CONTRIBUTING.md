# Contributing to Roxanne

Bug fixes, features and documentation improvements are welcome. This page says
how to build Roxanne, what to check before opening a pull request, and what a
good pull request looks like.

## Prerequisites

- Rust stable, installed with [rustup](https://rustup.rs).
- The minimum supported Rust version (MSRV) is the `rust-version` in
  `Cargo.toml`, currently 1.88. CI checks it, so do not use language or library
  features newer than that.

## Build and run

```bash
git clone https://github.com/Project-Colony/Roxanne.git
cd Roxanne
cargo run
```

## Before you open a pull request

Run the same checks as CI:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

If you touched the text buffer or the viewport, run the benchmarks too:

```bash
cargo bench --bench text_buffer --bench viewport -- --noplot
python scripts/check_benchmarks.py
```

## Pull requests

- One topic per pull request. Name the branch after the task, for example
  `fix/save-as-overwrite` or `feat/goto-symbol`.
- The pull request title is a [Conventional Commit](https://www.conventionalcommits.org)
  that reads as the changelog entry for users: `feat: ...`, `fix: ...`,
  `perf: ...`, `refactor: ...`, `docs: ...`, `test: ...`, `ci: ...`,
  `build: ...` or `chore: ...`.
- Every pull request is squash-merged, so its title becomes the commit on
  `main`, and release-please builds the changelog and the next version from
  those commits.
- If `main` moved, merge it into your branch before asking for a review.
- When a change alters behaviour, a path or a command, update `docs/` in the
  same pull request.

## Code style

- Format with `cargo fmt` (default rustfmt settings).
- No `unwrap()` outside tests: return an error or show it in the status bar.
- Add tests for any logic that is not trivial.
- Code, comments, documentation, commit messages and pull requests are written
  in English.

## Security

Do not report vulnerabilities in public issues. See [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under the
[GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).
