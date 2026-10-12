## What changed

Describe the change and why it is needed. Link the issue it fixes, if any.

## How it was verified

The commands you ran and what you tested by hand.

## Checklist

- [ ] The title is a Conventional Commit that reads as the changelog entry (`feat: ...`, `fix: ...`).
- [ ] `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --all` pass.
- [ ] Tests were added or updated for the changed logic.
- [ ] `docs/` is updated if behaviour, paths or commands changed.
