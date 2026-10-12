# Security Policy

Roxanne is a code editor. It opens and saves files anywhere you point it, reads
a project's `.roxanne.toml`, can load native plugins, and starts rust-analyzer
for Rust files. Security reports are taken seriously and handled first.

## Supported versions

Only the latest release receives security fixes. Until the first release,
fixes land on `main`.

## Reporting a vulnerability

Report vulnerabilities privately through GitHub: open the repository's
**Security** tab and click **Report a vulnerability**, or go straight to
<https://github.com/Project-Colony/Roxanne/security/advisories/new>. Only the
maintainers can read the report. Do not open a public issue for a bug that can
be exploited.

Please include:

- what an attacker can do, and under which conditions,
- the steps to reproduce,
- the release or commit you tested,
- a proof of concept, if you have one.

What to expect:

- an acknowledgement within three working days,
- a fix or a mitigation plan before any public disclosure, coordinated with you,
- credit in the release notes, if you want it.

## Scope

This policy covers the source code, the release binaries and the scripts in
this repository. Reports of particular interest:

- **Project configuration.** A `.roxanne.toml` comes with whatever folder you
  open, so it is untrusted. It can change the editor settings, the theme, the
  keymap and which built-in plugins run, and select one of your profiles. It
  cannot load native plugins: those only come from your user configuration.
  Anything that lets a project file load a library or run code is in scope.
- **rust-analyzer.** Roxanne starts it only when you use an LSP action on a
  saved Rust file, with build scripts, proc macros and check-on-save turned
  off. It still runs `cargo metadata`, and a project's `rust-analyzer.toml`,
  Cargo configuration and `rust-toolchain.toml` can change what it runs, so
  use the LSP actions only on code you trust. A way to make Roxanne run a
  project's code without an LSP action is in scope.
- **Saving.** Writes go through a temporary file and keep the target's
  permissions and symlinks. A save that loses data or writes somewhere other
  than the file being saved is in scope.

Out of scope: a native plugin you declared yourself (it runs with your rights
by design), and vulnerabilities in rust-analyzer itself, which belong upstream.
