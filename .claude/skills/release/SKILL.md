---
name: release
description: Prepare a new version release — bump Cargo.toml, scaffold UPDATES.md section, commit, and tag
disable-model-invocation: true
---

Prepare a release for the hex-terrain project. Accepts an optional version argument (e.g. `/release v0.0.4`). If no version is given, auto-increment the patch number from the latest git tag.

## Steps

1. **Determine version**
   - If the user provided a version argument, use it (ensure it starts with `v`)
   - Otherwise, run `git tag --sort=-v:refname | grep -m1 '^v[0-9]'` to find the latest tag, then increment the patch number
   - Confirm the version with the user before proceeding

2. **Validate UPDATES.md**
   - Read `UPDATES.md`
   - Check whether a `## <version>` section already exists
   - If it exists and has bullet items, skip to step 4
   - If it exists but is empty, tell the user to fill it in and stop
   - If it doesn't exist, proceed to step 3

3. **Scaffold UPDATES.md**
   - Insert a new `## <version>` section at the top (after the `# Updates` heading), with an empty bullet placeholder:
     ```
     ## v0.0.X

     - <describe changes here>
     ```
   - Tell the user to fill in the release notes and run `/release` again. **Stop here.**

4. **Bump Cargo.toml version**
   - Update the `version = "..."` field in `Cargo.toml` to match (without the `v` prefix)
   - Example: tag `v0.0.4` → `version = "0.0.4"`

5. **Verify build**
   - Run `cargo clippy -- -D warnings` — abort if it fails

6. **Commit and tag**
   - Stage `Cargo.toml` and `UPDATES.md`
   - Commit with message: `release <version>`
   - Create an annotated git tag: `git tag -a <version> -m "<version>"`
   - Show the final state with `git log --oneline -3` and `git tag --sort=-v:refname | head -5`

7. **Remind about push**
   - Tell the user: "Run `git push && git push --tags` to trigger the Pages deploy."
   - Do NOT push automatically
