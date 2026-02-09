---
name: cargo-check
description: Run cargo check and clippy to validate the build
---

Run `cargo check` followed by `cargo clippy -- -D warnings` in the project root.
Report any errors or warnings. If clean, confirm "Build OK, no warnings."
