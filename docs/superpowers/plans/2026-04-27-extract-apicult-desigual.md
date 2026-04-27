# Extract `apicult-desigual` to Standalone Repo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `crates/apicult-desigual` out of the `hex-terrain` workspace into its own public repo `dynnamitt/apicult-desigual-rs` (history preserved across the earlier `crates/hex-grid` rename) and switch `hex-terrain` to consume it as a tag-pinned git dependency.

**Architecture:** Two-pass `git filter-repo` extraction in a throwaway clone at `/tmp/apicult-extract`, then a fresh GitHub repo populated with `gh repo create --push`, then a feature-branch update of `hex-terrain` (Cargo.toml + delete crate dir + verification + anatomy cleanup) merged via PR.

**Tech Stack:** `git-filter-repo` (already installed), `gh` CLI (authed as `dynnamitt`), Cargo 2024 edition, Bevy 0.18 (for the consuming `hex-terrain` build verification).

**Spec:** [`docs/superpowers/specs/2026-04-27-extract-apicult-desigual-design.md`](../specs/2026-04-27-extract-apicult-desigual-design.md)

**Branch state at start:** A feature branch `extract-apicult-desigual` already exists in `hex-terrain` with the design doc committed. All Phase 3 commits add to this branch.

---

## Phase 1 — Extract history into a throwaway clone

### Task 1: Clone source repo into `/tmp/apicult-extract`

**Files:**
- Create: `/tmp/apicult-extract/` (fresh clone)

- [ ] **Step 1: Remove any prior leftover directory**

```bash
rm -rf /tmp/apicult-extract
```

- [ ] **Step 2: Clone**

```bash
git clone git@github.com:dynnamitt/hex-terrain.git /tmp/apicult-extract
```

Expected: clone succeeds, `/tmp/apicult-extract` exists with a `main` branch.

- [ ] **Step 3: Verify the two crate paths exist in history**

```bash
cd /tmp/apicult-extract
git log --oneline -- crates/hex-grid | wc -l
git log --oneline -- crates/apicult-desigual | wc -l
```

Expected: 18 and 1 respectively (or thereabouts — whatever exists in `hex-terrain` history at the time of the extraction).

---

### Task 2: Filter-repo Pass 1 — keep crate history, rename old path

**Files:**
- Modify: `/tmp/apicult-extract/` (history rewritten)

- [ ] **Step 1: Run filter-repo with path filter and path-rename**

```bash
cd /tmp/apicult-extract
git filter-repo \
  --path crates/hex-grid \
  --path crates/apicult-desigual \
  --path-rename crates/hex-grid:crates/apicult-desigual
```

Expected: filter-repo prints `Parsed N commits` then `New history written` and finishes with no error. `origin` remote is removed automatically. Commits not touching either path are dropped.

- [ ] **Step 2: Sanity-check the filtered tree**

```bash
git log --oneline | wc -l
ls crates/apicult-desigual
git log --all --oneline -- crates/apicult-desigual | head -5
```

Expected: ~19 commits remain, `crates/apicult-desigual/` contains `Cargo.toml src/ examples/ README.md`, and the log shows commits dating back further than the rename commit (the old `hex-grid` commits are now visible under the renamed path).

---

### Task 3: Filter-repo Pass 2 — lift crate to repo root

**Files:**
- Modify: `/tmp/apicult-extract/` (subdirectory promoted to root)

- [ ] **Step 1: Run filter-repo with `--subdirectory-filter`**

```bash
cd /tmp/apicult-extract
git filter-repo --subdirectory-filter crates/apicult-desigual --force
```

`--force` is required because filter-repo refuses to operate on an already-filtered repo by default.

Expected: succeeds. Files now live at the repo root.

- [ ] **Step 2: Verify root contents**

```bash
ls
cat Cargo.toml | head -3
git log --oneline | wc -l
```

Expected: root contains `Cargo.toml src/ examples/ README.md`. `Cargo.toml` shows `name = "apicult-desigual"`. Commit count unchanged from end of Task 2.

---

### Task 4: Polish standalone crate

The crate's docs and example doc-comments still use workspace-relative `cargo run -p apicult-desigual` syntax that won't work in a standalone repo. Add a `.gitignore` too.

**Files:**
- Modify: `/tmp/apicult-extract/README.md` (lines 41–44)
- Modify: `/tmp/apicult-extract/examples/geo_export.rs` (lines 4–8)
- Create: `/tmp/apicult-extract/.gitignore`

- [ ] **Step 1: Edit README.md — strip `-p apicult-desigual` from invocations**

Replace each occurrence of `cargo run -p apicult-desigual --example geo_export` with `cargo run --example geo_export`. There are 4 such lines in `README.md` (lines 41–44).

```bash
cd /tmp/apicult-extract
sed -i 's/cargo run -p apicult-desigual --example geo_export/cargo run --example geo_export/g' README.md
```

Verify:

```bash
grep -n "cargo run" README.md
```

Expected: 4 lines, all starting with `cargo run --example geo_export` (no `-p` flag).

- [ ] **Step 2: Edit examples/geo_export.rs — same replacement in doc comments**

```bash
sed -i 's|cargo run -p apicult-desigual --example geo_export|cargo run --example geo_export|g' examples/geo_export.rs
```

Verify:

```bash
grep -n "cargo run" examples/geo_export.rs
```

Expected: 5 lines, all starting with `//! cargo run --example geo_export`.

- [ ] **Step 3: Create `.gitignore`**

Write `/tmp/apicult-extract/.gitignore` with:

```
/target
Cargo.lock
```

(Library crates conventionally do not commit `Cargo.lock`.)

- [ ] **Step 4: Commit polish**

```bash
git add README.md examples/geo_export.rs .gitignore
git commit -m "chore: adapt invocations and add .gitignore for standalone repo"
```

---

### Task 5: Verify standalone build

**Files:**
- (none — build verification only)

- [ ] **Step 1: Build**

```bash
cd /tmp/apicult-extract
cargo build
```

Expected: clean build of `apicult-desigual v0.1.0`. Dependencies (`glam`, `hexx`, `noise`) resolve from crates.io.

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all tests pass. (Tests live in `src/math.rs`, `src/serialize.rs`, etc., per the existing crate layout.)

- [ ] **Step 3: Smoke-test the example**

```bash
cargo run --example geo_export -- 3 1.0 --format json-v2 | head -5
```

Expected: JSON output starting with `{"version":2,"tris":...`.

If any of these fail, stop and investigate before proceeding to Phase 2 — the new repo must build before we publish it.

---

## Phase 2 — Create the GitHub repo and tag

### Task 6: Create `dynnamitt/apicult-desigual-rs` and push

**Files:**
- (creates the GitHub repo)

- [ ] **Step 1: Create + push in one command**

```bash
cd /tmp/apicult-extract
gh repo create dynnamitt/apicult-desigual-rs \
  --public \
  --description "Hex grid layout + serialization library (Rust). Used by hex-terrain." \
  --source=. \
  --push
```

Expected: repo created at `https://github.com/dynnamitt/apicult-desigual-rs`, `main` branch pushed, `origin` remote set.

- [ ] **Step 2: Verify**

```bash
gh repo view dynnamitt/apicult-desigual-rs --json visibility,defaultBranchRef
git remote -v
```

Expected: `visibility: PUBLIC`, default branch `main`. `origin` points at `git@github.com:dynnamitt/apicult-desigual-rs.git` (or `https://...`).

---

### Task 7: Tag and push `v0.1.0`

**Files:**
- (creates a git tag)

- [ ] **Step 1: Create annotated tag**

```bash
cd /tmp/apicult-extract
git tag -a v0.1.0 -m "v0.1.0 — initial release as standalone crate"
```

- [ ] **Step 2: Push the tag**

```bash
git push origin v0.1.0
```

- [ ] **Step 3: Verify**

```bash
gh release list --repo dynnamitt/apicult-desigual-rs
git ls-remote --tags origin
```

Expected: tag `v0.1.0` visible on the remote. (No GitHub Release object yet — just the tag. A Release can be added later if desired.)

---

## Phase 3 — Update `hex-terrain` to consume via git

All Phase 3 work happens on the existing `extract-apicult-desigual` branch in `/var/home/kdm/code/hex-terrain`, which already contains the design-doc commit.

### Task 8: Update workspace `Cargo.toml`

**Files:**
- Modify: `/var/home/kdm/code/hex-terrain/Cargo.toml` (lines 2 and 57)

- [ ] **Step 1: Switch to feature branch**

```bash
cd /var/home/kdm/code/hex-terrain
git checkout extract-apicult-desigual
git status
```

Expected: clean tree on `extract-apicult-desigual`.

- [ ] **Step 2: Edit `Cargo.toml` — remove crate from workspace members**

Change line 2 from:

```toml
members = ["crates/mesh-gradient", "crates/apicult-desigual", "crates/flora"]
```

to:

```toml
members = ["crates/mesh-gradient", "crates/flora"]
```

- [ ] **Step 3: Edit `Cargo.toml` — switch dep to git+tag**

Change line 57 from:

```toml
apicult-desigual = { path = "crates/apicult-desigual" }
```

to:

```toml
apicult-desigual = { git = "https://github.com/dynnamitt/apicult-desigual-rs.git", tag = "v0.1.0" }
```

- [ ] **Step 4: Verify the diff**

```bash
git diff Cargo.toml
```

Expected: only the two lines above changed.

---

### Task 9: Remove the local crate directory

**Files:**
- Delete: `/var/home/kdm/code/hex-terrain/crates/apicult-desigual/`

- [ ] **Step 1: Remove the directory**

```bash
cd /var/home/kdm/code/hex-terrain
git rm -r crates/apicult-desigual
```

Expected: every file under `crates/apicult-desigual/` shows `deleted:` in `git status`.

- [ ] **Step 2: Confirm hex-terrain source files still reference the crate by name (unchanged imports)**

```bash
grep -rn "apicult_desigual\|apicult-desigual" src/ Cargo.toml | head
```

Expected: hits in `src/h_terrain.rs`, `src/h_terrain/h_grid_layout.rs`, `src/h_terrain/gaps.rs`, `src/h_terrain/startup_systems.rs`, `src/h_terrain/tests.rs`, `src/h_terrain/flora_spawn.rs`, plus the new git-dep entry in `Cargo.toml`. Source code is untouched — only the dependency source changed.

---

### Task 10: Verify `hex-terrain` builds against the git dependency

**Files:**
- Modify: `Cargo.lock` (regenerated)

- [ ] **Step 1: Build (this fetches the git dep)**

```bash
cd /var/home/kdm/code/hex-terrain
cargo build
```

Expected: Cargo clones `apicult-desigual-rs@v0.1.0` into `~/.cargo/git/`, then builds the workspace. Build succeeds.

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all tests pass, including `h_terrain::tests` which exercise `apicult_desigual::gap_filler`.

- [ ] **Step 3: Build WASM**

```bash
make wasm
```

Expected: `make wasm` succeeds. (If `wasm32` target or `wasm-bindgen-cli` is missing, the Makefile auto-installs them per `make wasm-deps`.)

- [ ] **Step 4: Verify `Cargo.lock` updated**

```bash
git diff Cargo.lock | grep -A3 "apicult-desigual" | head -20
```

Expected: `apicult-desigual` entry now shows `source = "git+https://github.com/dynnamitt/apicult-desigual-rs.git?tag=v0.1.0#<sha>"` instead of an implicit path source.

If any verification fails, stop and investigate. Do not commit a broken state.

---

### Task 11: Update `.wolf/anatomy.md`

The anatomy file still references the old `crates/hex-grid/` paths (it was never updated after the rename, and now the directory is leaving the repo entirely). Remove those sections.

**Files:**
- Modify: `/var/home/kdm/code/hex-terrain/.wolf/anatomy.md`

- [ ] **Step 1: Find the sections to remove**

```bash
grep -n "^## crates/hex-grid\|^## crates/apicult-desigual" .wolf/anatomy.md
```

Expected: lines `## crates/hex-grid/`, `## crates/hex-grid/examples/`, `## crates/hex-grid/src/` (per current state — anatomy uses the pre-rename name).

- [ ] **Step 2: Delete those sections**

Delete each `## crates/hex-grid/...` heading and all its bullet entries up to (but not including) the next `## ` heading. Use the `Edit` tool with `replace_all: false` for each section, or open the file and remove the lines manually.

- [ ] **Step 3: Also fix the stale `svg-preview.html` mention if it references `hex-grid`**

```bash
grep -n "hex-grid" .wolf/anatomy.md
```

If any `hex-grid` strings remain (e.g. `svg-preview.html — hex-grid preview`), update them to `apicult-desigual` or just leave the human-written description as-is — the auto-scan will refresh on the next run.

- [ ] **Step 4: Verify**

```bash
grep -n "apicult\|hex-grid" .wolf/anatomy.md
```

Expected: no `crates/hex-grid` or `crates/apicult-desigual` paths remain. The design-doc reference at line ~110 is fine to keep.

---

### Task 12: Commit and push the feature branch

**Files:**
- (commit everything staged)

- [ ] **Step 1: Stage**

```bash
cd /var/home/kdm/code/hex-terrain
git add Cargo.toml Cargo.lock .wolf/anatomy.md
git status
```

Expected staged set:
- modified: `Cargo.toml`
- modified: `Cargo.lock`
- modified: `.wolf/anatomy.md`
- deleted: every file under `crates/apicult-desigual/`

- [ ] **Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
chore: extract apicult-desigual to standalone repo

The apicult-desigual crate now lives at
https://github.com/dynnamitt/apicult-desigual-rs and is consumed
as a tag-pinned git dependency. History was preserved across the
earlier hex-grid rename via git-filter-repo.

- drop crates/apicult-desigual from workspace members
- switch dep from path to git tag v0.1.0
- remove crates/apicult-desigual/ directory
- clean up stale crates/hex-grid entries in .wolf/anatomy.md
EOF
)"
```

- [ ] **Step 3: Push**

```bash
git push -u origin extract-apicult-desigual
```

Expected: branch pushed, gh prints PR-create suggestion.

---

### Task 13: Open the PR

**Files:**
- (creates a GitHub PR)

- [ ] **Step 1: Create PR**

```bash
gh pr create --title "Extract apicult-desigual to standalone repo" --body "$(cat <<'EOF'
## Summary
- Moved `crates/apicult-desigual` out of the workspace into a new public repo: https://github.com/dynnamitt/apicult-desigual-rs
- History preserved across the earlier `crates/hex-grid` rename (via `git-filter-repo`)
- `hex-terrain` now consumes it as a tag-pinned git dependency (`v0.1.0`)
- `mesh-gradient` and `flora` remain as workspace crates

## Test plan
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (including `h_terrain::tests` that use `apicult_desigual::gap_filler`)
- [ ] `make wasm` succeeds
- [ ] Visual sanity check: `cargo run` starts up and the terrain renders
EOF
)"
```

- [ ] **Step 2: Confirm PR URL**

The PR URL is printed by `gh pr create`. Note it for the user.

---

## Done

After Task 13, all design-doc objectives are met:
- `dynnamitt/apicult-desigual-rs` exists, public, with preserved history and a `v0.1.0` tag
- `hex-terrain` PR is open with the workspace member removed and a tag-pinned git dep in its place
- `mesh-gradient` and `flora` are untouched

Merging the PR is the user's call (per project rule "never merge directly to main" — done via the GitHub UI).
