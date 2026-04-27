# Extract `apicult-desigual` to Standalone Repo — Design

**Date:** 2026-04-27
**Status:** Approved (design phase); awaiting implementation plan.

## Goal

Move the `apicult-desigual` crate (currently `crates/apicult-desigual` in the
`hex-terrain` workspace) into its own public GitHub repository at
`dynnamitt/apicult-desigual-rs`, with full git history preserved across the
earlier `crates/hex-grid` rename. The `hex-terrain` crate continues to consume
it as a tag-pinned git dependency. The other workspace crates (`mesh-gradient`,
`flora`) stay in `hex-terrain`.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| History preservation | **Preserve** via `git filter-repo` | Keep `git log` / `git blame` working; user explicitly chose. |
| Consumption model | **Git dependency, tag-pinned** | Simple, explicit versioning, no submodule friction. |
| Repo name | `apicult-desigual-rs` | Rust convention (`-rs` suffix); leaves room for future ports in other languages. |
| Crate name | unchanged: `apicult-desigual` | crates.io is Rust-only; suffix would be redundant in `Cargo.toml` and `use` paths. |
| Visibility | Public | User confirmed. |
| Owner | `dynnamitt` (personal account) | Matches existing `hex-terrain` ownership. |
| Other workspace crates | Stay in `hex-terrain` | Only `apicult-desigual` is being extracted. |

## Architecture

Two repositories after extraction:

```
dynnamitt/apicult-desigual-rs                 dynnamitt/hex-terrain
└── (crate at root)                           ├── Cargo.toml — git dep on apicult-desigual-rs@v0.1.0
    ├── Cargo.toml                            ├── crates/
    ├── src/                                  │   ├── mesh-gradient/   (stays)
    ├── examples/                             │   └── flora/           (stays)
    └── README.md                             └── src/...
```

`hex-terrain`'s use of `apicult_desigual::*` is unchanged at the source level —
only the dependency declaration in `Cargo.toml` changes from `path` to `git`.

## Implementation Outline

### Phase 1 — Extract history into the new repo

Done in a throwaway clone at `/tmp/apicult-extract` so the source repo is
never modified. Prerequisite `git-filter-repo` is already installed.

1. `git clone git@github.com:dynnamitt/hex-terrain.git /tmp/apicult-extract`
2. From that clone, run two filter-repo passes:
   - **Pass 1** keeps only commits touching `crates/hex-grid` or
     `crates/apicult-desigual`, and rewrites the old path to the new name so
     history looks unified:
     ```bash
     git filter-repo \
       --path crates/hex-grid \
       --path crates/apicult-desigual \
       --path-rename crates/hex-grid:crates/apicult-desigual
     ```
   - **Pass 2** lifts the crate from `crates/apicult-desigual/` to the repo root:
     ```bash
     git filter-repo --subdirectory-filter crates/apicult-desigual
     ```
3. Result: 19 commits, files at the repo root, original authors and dates
   preserved. (SHAs change — filter-repo always rewrites — but message,
   author, and date survive.)

### Phase 2 — Create and populate the GitHub repo

1. `gh repo create dynnamitt/apicult-desigual-rs --public --source=/tmp/apicult-extract --push`
2. Update the new repo's `README.md` to remove workspace-relative invocations:
   - `cargo run -p apicult-desigual --example geo_export` → `cargo run --example geo_export`
3. Verify the standalone build: `cargo build && cargo test`.
4. Tag and push `v0.1.0`.

### Phase 3 — Update `hex-terrain` to consume via git (feature branch + PR)

1. Create a feature branch (`extract-apicult-desigual` or similar).
2. `Cargo.toml`:
   - Remove `"crates/apicult-desigual"` from `workspace.members`.
   - Change `apicult-desigual = { path = "crates/apicult-desigual" }` to
     `apicult-desigual = { git = "https://github.com/dynnamitt/apicult-desigual-rs", tag = "v0.1.0" }`.
3. `rm -rf crates/apicult-desigual`.
4. Run verification: `cargo build`, `cargo test`, `make wasm`.
5. Update `CLAUDE.md` (drop the `crates/apicult-desigual/` description and
   adjust the workspace-member listing).
6. Update `.wolf/anatomy.md` (remove file entries under
   `crates/apicult-desigual/`).
7. Commit, push, open PR via `gh pr create`. Do **not** merge locally —
   project rule is "never merge directly to main".

## Verification

- New repo: `cargo build && cargo test` from a fresh clone of
  `apicult-desigual-rs` succeeds.
- `hex-terrain` PR: `cargo build`, `cargo test`, and `make wasm` all pass
  with the git dependency resolved.
- `git log` in the new repo shows commits dating back to the original
  `crates/hex-grid` work (rewritten path).

## Risks and Mitigations

- **Repo rename or delete breaks `hex-terrain`'s build.** Mitigated by tag
  pinning (failure is loud and the URL is the obvious place to look). Also
  mitigated by the fact that the same person owns both repos.
- **filter-repo SHA rewrite invalidates any external refs to old crate
  commits.** No known external consumers exist; this is acceptable.
- **`Cargo.lock` churn in `hex-terrain`.** Expected — review the diff to
  confirm only `apicult-desigual` and its transitive deps move from path
  to git source.

## Out of Scope

- Publishing `apicult-desigual` to crates.io (could be done later).
- Setting up CI in the new repo (could be done later).
- Migrating `mesh-gradient` or `flora` (explicitly staying).
