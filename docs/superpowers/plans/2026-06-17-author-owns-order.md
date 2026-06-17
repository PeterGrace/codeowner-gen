# Author-Owns-Order Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `codeowner-gen`'s heuristic path-sorting with an author-owned ordering model: every entry has a group (default `ungrouped`), the `ungrouped` block renders first, other group blocks render alphabetically by name, and within a block author order is preserved (team-derived lines fall back to alphabetical-by-path).

**Architecture:** Introduce one composite sort (`structs::sort_entries`) keyed on `(ungrouped-first, group-name, author-index, path)`, a pure renderer (`render::render_body`) that emits contiguous `BEGIN`/`END` blocks, and a pure pipeline function (`pipeline::process`) that captures author order before merging then sorts. `main.rs` becomes thin orchestration. The existing path `Ord` is kept but demoted to the within-block fallback tiebreaker.

**Tech Stack:** Rust 2024 edition, serde / serde_yaml, anyhow, clap. Tests are `#[cfg(test)]` modules plus the `src/tests/` directory. Build/test with `cargo`.

**Reference spec:** `docs/superpowers/specs/2026-06-17-author-owns-order-design.md`

---

## File Structure

- **Modify `src/structs.rs`** — add `CodeOwner::group_name()` helper and `sort_entries()` (the composite sort). Keep the existing `Ord` impl (now the fallback tiebreaker).
- **Create `src/render.rs`** — pure `render_body(&[CodeOwner]) -> String` plus a private `block_label()`. Owns all output-string formatting.
- **Create `src/pipeline.rs`** — pure `process(CodeOwners) -> Result<Vec<CodeOwner>, String>`: capture author order → expand aliases → merge teams → sort.
- **Modify `src/main.rs`** — register `mod render;` / `mod pipeline;`, replace the inline longest-path/group-flag/double-sort/write-loop block with `pipeline::process` + `render::render_body`.
- **Create `src/tests/rendering.rs`** + register in `src/tests/mod.rs` — end-to-end YAML→string test.
- **Modify `codeowners.yaml`** — refresh the sample to exercise the new model.
- **Modify `README.md`** — document the ordering model.

---

### Task 1: `group_name()` helper + `sort_entries()`

**Files:**
- Modify: `src/structs.rs` (add helper near the `Ord` impl ~line 138; add tests inside the existing `#[cfg(test)] mod tests` ~line 140)

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `mod tests` block in `src/structs.rs` (after the existing `test_negated_subpath_sorts_after_parent` test). Add a helper at the top of the test module too.

```rust
    use std::collections::HashMap;

    fn grouped_entry(path: &str, group: &str) -> CodeOwner {
        CodeOwner {
            path: PathBuf::from(path),
            negate: false,
            owners: vec![],
            comment: None,
            group: Some(group.to_string()),
        }
    }

    fn author_order(paths: &[&str]) -> HashMap<PathBuf, usize> {
        paths
            .iter()
            .enumerate()
            .map(|(i, p)| (PathBuf::from(*p), i))
            .collect()
    }

    fn paths_of(entries: &[CodeOwner]) -> Vec<&str> {
        entries.iter().map(|e| e.path.to_str().unwrap()).collect()
    }

    #[test]
    fn test_ungrouped_block_sorts_first() {
        let order = author_order(&["src/", "*", "docs/"]);
        let mut entries = vec![
            grouped_entry("src/", "core"),
            entry("*"),
            grouped_entry("docs/", "docs"),
        ];
        sort_entries(&mut entries, &order);
        // ungrouped (*) first, then groups alphabetically: core, docs
        assert_eq!(paths_of(&entries), vec!["*", "src/", "docs/"]);
    }

    #[test]
    fn test_groups_sort_alphabetically_by_name() {
        let order = author_order(&["z.txt", "a.txt"]);
        let mut entries = vec![
            grouped_entry("z.txt", "zeta"),
            grouped_entry("a.txt", "alpha"),
        ];
        sort_entries(&mut entries, &order);
        // alpha block before zeta block, regardless of author order
        assert_eq!(paths_of(&entries), vec!["a.txt", "z.txt"]);
    }

    #[test]
    fn test_within_group_preserves_author_order() {
        // author wrote src/b before src/a; author owns order, NOT alphabetical
        let order = author_order(&["src/b", "src/a"]);
        let mut entries = vec![
            grouped_entry("src/a", "core"),
            grouped_entry("src/b", "core"),
        ];
        sort_entries(&mut entries, &order);
        assert_eq!(paths_of(&entries), vec!["src/b", "src/a"]);
    }

    #[test]
    fn test_team_derived_entries_fall_back_to_path_order() {
        // only src/main has an authored position; the others come from teams
        let order = author_order(&["src/main"]);
        let mut entries = vec![
            grouped_entry("src/z", "core"),
            grouped_entry("src/main", "core"),
            grouped_entry("src/a", "core"),
        ];
        sort_entries(&mut entries, &order);
        // authored first, then team-derived alphabetical-by-path
        assert_eq!(paths_of(&entries), vec!["src/main", "src/a", "src/z"]);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib sort_entries 2>&1; cargo test --lib group 2>&1 | head -40`
Expected: compile error — `cannot find function `sort_entries``. (That counts as failing.)

- [ ] **Step 3: Add the `group_name()` helper and `sort_entries()`**

In `src/structs.rs`, immediately after the `impl Ord for CodeOwner { ... }` block (around line 138), add:

```rust
impl CodeOwner {
    /// The effective group name: the explicit group, or the reserved
    /// `"ungrouped"` name when none was specified.
    pub(crate) fn group_name(&self) -> &str {
        self.group.as_deref().unwrap_or("ungrouped")
    }
}

/// Sorts entries into render order under the "author owns order" model.
///
/// Key: (ungrouped-first, group name alphabetical, authored index, path Ord).
/// `author_order` maps a path to its index in the original `entries:` list;
/// paths absent from the map (team-derived) sort after authored entries and
/// fall back to the path `Ord` tiebreaker.
pub(crate) fn sort_entries(
    entries: &mut [CodeOwner],
    author_order: &std::collections::HashMap<std::path::PathBuf, usize>,
) {
    entries.sort_by(|a, b| {
        let a_ungrouped = a.group_name() == "ungrouped";
        let b_ungrouped = b.group_name() == "ungrouped";
        // false (ungrouped) sorts before true (grouped): invert so ungrouped is first
        (!a_ungrouped)
            .cmp(&(!b_ungrouped))
            .then_with(|| a.group_name().cmp(b.group_name()))
            .then_with(|| {
                let ai = author_order.get(&a.path).copied().unwrap_or(usize::MAX);
                let bi = author_order.get(&b.path).copied().unwrap_or(usize::MAX);
                ai.cmp(&bi)
            })
            .then_with(|| a.cmp(b))
    });
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS — all four new tests plus the existing `structs` tests (including `test_wildcard_sorts_before_regular_paths`, `test_negated_*`) are green.

- [ ] **Step 5: Commit**

```bash
git add src/structs.rs
git commit -m "feat: add group_name helper and author-owned sort_entries

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `render.rs` — pure renderer

**Files:**
- Create: `src/render.rs`

- [ ] **Step 1: Write the failing test**

Create `src/render.rs` with the test module first (implementation added in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{CodeOwner, Owner};
    use std::path::PathBuf;

    fn co(path: &str, group: Option<&str>, negate: bool, comment: Option<&str>, owner: Option<&str>) -> CodeOwner {
        CodeOwner {
            path: PathBuf::from(path),
            negate,
            owners: owner.map(|o| vec![Owner::Team(o.to_string())]).unwrap_or_default(),
            comment: comment.map(|c| c.to_string()),
            group: group.map(|g| g.to_string()),
        }
    }

    // Returns just the block header/footer lines, in order.
    fn structure(body: &str) -> Vec<&str> {
        body.lines()
            .filter(|l| l.starts_with("#######") || l.starts_with("### END"))
            .collect()
    }

    #[test]
    fn test_render_emits_contiguous_blocks_with_matching_footers() {
        // already in final sorted order: ungrouped, then core, then docs
        let entries = vec![
            co("*", None, false, None, Some("@org/default")),
            co("src/", Some("core"), false, Some("core code"), Some("@org/core")),
            co("lib/", Some("core"), false, None, Some("@org/core")),
            co("build/", Some("docs"), true, None, None),
        ];
        let body = render_body(&entries);

        assert_eq!(
            structure(&body),
            vec![
                "####### BEGIN UNGROUPED",
                "### END UNGROUPED",
                "####### BEGIN GROUP CORE",
                "### END GROUP CORE",
                "####### BEGIN GROUP DOCS",
                "### END GROUP DOCS",
            ]
        );
        // comment renders above its entry
        assert!(body.contains("# core code\n"));
        // negate prefix renders
        assert!(body.lines().any(|l| l.starts_with("!build/")));
        // owners render on the catch-all line
        assert!(body.lines().any(|l| l.starts_with("*") && l.contains("@org/default")));
    }

    #[test]
    fn test_explicit_ungrouped_group_uses_ungrouped_block() {
        let entries = vec![co("x", Some("ungrouped"), false, None, Some("@org/x"))];
        let body = render_body(&entries);
        assert_eq!(structure(&body), vec!["####### BEGIN UNGROUPED", "### END UNGROUPED"]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib render 2>&1 | head -30`
Expected: compile error — `cannot find function `render_body``.

- [ ] **Step 3: Write the implementation**

Add this **above** the test module in `src/render.rs`:

```rust
use crate::structs::CodeOwner;

/// Renders the body of the CODEOWNERS file (everything after the version
/// header). Entries MUST already be sorted into final render order. Blocks are
/// driven entirely by the entries present: a header opens when the block label
/// changes, and every opened block is closed with a matching footer.
pub(crate) fn render_body(entries: &[CodeOwner]) -> String {
    let longest_path = entries
        .iter()
        .map(|c| c.path.as_os_str().len() + if c.negate { 1 } else { 0 })
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let mut current: Option<String> = None;

    for co in entries {
        let label = block_label(co);
        if current.as_deref() != Some(label.as_str()) {
            if let Some(prev) = current.take() {
                out.push_str(&format!("### END {}\n", prev));
            }
            out.push_str(&format!("####### BEGIN {}\n", label));
            current = Some(label);
        }

        if let Some(comment) = &co.comment {
            out.push_str(&format!("# {}\n", comment));
        }

        let display_path = if co.negate {
            format!("!{}", co.path.display())
        } else {
            co.path.display().to_string()
        };

        let mut owners = String::new();
        for o in &co.owners {
            owners = format!("{} {}", owners, o);
        }

        out.push_str(&format!(
            "{:width$} {}\n",
            display_path,
            owners,
            width = longest_path
        ));
    }

    if let Some(prev) = current.take() {
        out.push_str(&format!("### END {}\n", prev));
    }

    out
}

/// The block label for an entry. The reserved `ungrouped` name (explicit or
/// implied) renders as the `UNGROUPED` block; everything else as `GROUP <NAME>`.
fn block_label(co: &CodeOwner) -> String {
    let name = co.group_name();
    if name == "ungrouped" {
        "UNGROUPED".to_string()
    } else {
        format!("GROUP {}", name.to_ascii_uppercase())
    }
}
```

- [ ] **Step 4: Register the module so the test compiles**

In `src/main.rs`, add `mod render;` next to the other `mod` declarations (after `mod owner_groups;`, ~line 4):

```rust
mod render;
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib render`
Expected: PASS — both render tests green.

- [ ] **Step 6: Commit**

```bash
git add src/render.rs src/main.rs
git commit -m "feat: add pure render_body for contiguous group blocks

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: `pipeline.rs` — pure process function + end-to-end test

**Files:**
- Create: `src/pipeline.rs`
- Create: `src/tests/rendering.rs`
- Modify: `src/tests/mod.rs`
- Modify: `src/main.rs` (register `mod pipeline;`)

- [ ] **Step 1: Write the failing end-to-end test**

Create `src/tests/rendering.rs`:

```rust
use crate::pipeline::process;
use crate::render::render_body;
use crate::structs::CodeOwners;

fn structure(body: &str) -> Vec<&str> {
    body.lines()
        .filter(|l| l.starts_with("#######") || l.starts_with("### END"))
        .collect()
}

// Index of the line whose first whitespace-delimited token equals `path`.
fn line_index(body: &str, path: &str) -> usize {
    body.lines()
        .position(|l| l.split_whitespace().next() == Some(path))
        .unwrap_or_else(|| panic!("path {} not found in:\n{}", path, body))
}

#[test]
fn test_end_to_end_ordering_and_blocks() {
    const INPUT: &str = "
---
entries:
  - path: \"*\"
    owners: [\"@org/default\"]
  - path: \"src/\"
    group: \"core\"
    owners: [\"@org/core\"]
  - path: \"docs/\"
    group: \"docs\"
    owners: [\"@org/writers\"]
  - path: \"src/api/\"
    group: \"core\"
    owners: [\"@org/api\"]
teams:
  \"@org/legacy\":
    - path: \"src/legacy/\"
      group: \"core\"
";
    let code_owners = serde_yaml::from_str::<CodeOwners>(INPUT).unwrap();
    let entries = process(code_owners).unwrap();
    let body = render_body(&entries);

    // ungrouped first, then groups alphabetically (core before docs)
    assert_eq!(
        structure(&body),
        vec![
            "####### BEGIN UNGROUPED",
            "### END UNGROUPED",
            "####### BEGIN GROUP CORE",
            "### END GROUP CORE",
            "####### BEGIN GROUP DOCS",
            "### END GROUP DOCS",
        ]
    );

    // ungrouped block precedes the core block
    assert!(line_index(&body, "*") < line_index(&body, "src/"));
    // within core: authored order (src/, src/api/) then team-derived (src/legacy/)
    assert!(line_index(&body, "src/") < line_index(&body, "src/api/"));
    assert!(line_index(&body, "src/api/") < line_index(&body, "src/legacy/"));
    // core block precedes docs block
    assert!(line_index(&body, "src/legacy/") < line_index(&body, "docs/"));
}
```

- [ ] **Step 2: Register the test module**

In `src/tests/mod.rs`, add:

```rust
mod rendering;
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --lib test_end_to_end_ordering_and_blocks 2>&1 | head -30`
Expected: compile error — `unresolved import `crate::pipeline``.

- [ ] **Step 4: Write the pipeline implementation**

Create `src/pipeline.rs`:

```rust
use crate::structs::{CodeOwner, CodeOwners, sort_entries};
use crate::{owner_groups, teams};
use std::collections::HashMap;
use std::path::PathBuf;

/// Resolves a parsed config into the final, render-ordered list of entries.
///
/// Steps: capture each entry's authored position (its index in the original
/// `entries:` list) BEFORE merging destroys order, expand `owner_groups`
/// aliases, merge the `teams:` shorthand, then sort under the author-owned
/// ordering model.
pub(crate) fn process(code_owners: CodeOwners) -> Result<Vec<CodeOwner>, String> {
    let mut author_order: HashMap<PathBuf, usize> = HashMap::new();
    for (i, entry) in code_owners.entries.iter().enumerate() {
        author_order.entry(entry.path.clone()).or_insert(i);
    }

    let (expanded_entries, expanded_teams) = owner_groups::expand_owner_groups(
        &code_owners.owner_groups,
        code_owners.entries,
        code_owners.teams,
    )?;

    let mut entries = teams::merge_teams_into_entries(expanded_entries, expanded_teams);
    sort_entries(&mut entries, &author_order);
    Ok(entries)
}
```

- [ ] **Step 5: Register the module**

In `src/main.rs`, add `mod pipeline;` next to the other `mod` declarations (~line 4):

```rust
mod pipeline;
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test --lib test_end_to_end_ordering_and_blocks`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/pipeline.rs src/tests/rendering.rs src/tests/mod.rs src/main.rs
git commit -m "feat: add pipeline::process with author-order capture + e2e test

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Wire `main.rs` to the new pipeline + renderer

**Files:**
- Modify: `src/main.rs:57-132` (replace the expand/merge/longest-path/sort/write-loop block)
- Modify: `codeowners.yaml`

- [ ] **Step 1: Replace the body from merge through file write**

In `src/main.rs`, replace everything from the current expand block (the `let (expanded_entries, expanded_teams) = ...` at ~line 59) through the end of the write loop (the closing `}` before `Ok(())` at ~line 131) with:

```rust
    let entries = match pipeline::process(code_owners) {
        Ok(v) => v,
        Err(err) => bail!("Invalid owner group reference: {}", err),
    };

    // And now, to write the file.
    let mut fd = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&matches.output_file)?;
    fd.write_all(
        format!(
            "# Generated by https://github.com/PeterGrace/codeowner-gen v{}/{}\n#\n",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_HASH")
        )
        .as_bytes(),
    )?;
    fd.write_all(render::render_body(&entries).as_bytes())?;
    Ok(())
```

Note: `code_owners` no longer needs `mut`. Change `let mut code_owners = match serde_yaml::from_str...` (~line 53) to `let code_owners = match serde_yaml::from_str...`.

- [ ] **Step 2: Build and confirm no dead code / unused warnings**

Run: `cargo build 2>&1`
Expected: builds clean. (`teams`/`owner_groups` are now used via `pipeline`; the old `grouped`/`longest_path` locals are gone.)

- [ ] **Step 3: Refresh the sample config**

Replace the contents of `codeowners.yaml` with a sample that exercises the new model (interleaved groups + ungrouped + negate + a team-derived path):

```yaml
---
entries:
  - path: "*"
    comment: "Default owner for everything not otherwise claimed"
    owners:
      - "@petergrace"
  - path: "src/"
    group: "core"
    owners:
      - "@petergrace"
  - path: "docs/"
    group: "docs"
    owners:
      - "pete.grace@gmail.com"
  - path: "src/internal/"
    group: "core"
    owners:
      - "@petergrace/teamname"
  - path: "docs/generated/"
    group: "docs"
    negate: true
    owners: []
teams:
  "@petergrace":
    - path: "src/legacy/"
      group: "core"
```

- [ ] **Step 4: Run the binary and verify the output layout**

Run: `cargo run -- codeowners.yaml -o /tmp/CODEOWNERS && cat /tmp/CODEOWNERS`
Expected output (column padding may differ; the structure is what matters):

```
# Generated by https://github.com/PeterGrace/codeowner-gen vX/Y
#
####### BEGIN UNGROUPED
# Default owner for everything not otherwise claimed
*                 @petergrace
### END UNGROUPED
####### BEGIN GROUP CORE
src/              @petergrace
src/internal/     @petergrace/teamname
src/legacy/       @petergrace
### END GROUP CORE
####### BEGIN GROUP DOCS
docs/             pete.grace@gmail.com
!docs/generated/
### END GROUP DOCS
```

Verify by eye: `UNGROUPED` first; `CORE` before `DOCS` (alphabetical); within `CORE`, authored `src/` and `src/internal/` precede team-derived `src/legacy/`; every block has a matching `### END`.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: PASS — all unit tests, serialization tests, and the e2e test.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs codeowners.yaml
git commit -m "feat: render CODEOWNERS via author-owned pipeline; refresh sample

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Documentation + final lint/format pass

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the README ordering documentation**

Find the section of `README.md` that describes ordering/grouping (search for `group`, `sort`, or `UNGROUPED`). Replace/augment it to state the new model. Add a subsection like:

```markdown
## Ordering and grouping

`codeowner-gen` follows an **author-owns-order** model. Because GitHub
CODEOWNERS resolves ownership by *last matching pattern wins*, the order of
lines is meaningful — so the tool never silently reorders your precedence list.

- **Every entry belongs to a group.** If you omit `group:`, the entry is placed
  in the implicit, reserved `ungrouped` group.
- **Block order:** the `ungrouped` block is emitted first (it forms a
  low-precedence baseline), followed by every other group block sorted
  **alphabetically by group name**.
- **Within a block:** entries keep the order you wrote them in. Entries that
  exist only because of the `teams:` mapping (which is an unordered map) have no
  authored position, so they are appended after the authored entries and sorted
  alphabetically by path.

To make an entry win over everything else, place it last within the
last-rendered block — there is no separate priority field.
```

If no such section exists, add the above as a new top-level section before the examples.

- [ ] **Step 2: Format and lint**

Run: `cargo fmt && cargo clippy --all-targets 2>&1 | tail -20`
Expected: `cargo fmt` makes no/minimal changes; `cargo clippy` reports no warnings.

- [ ] **Step 3: Final full test run**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add README.md src/
git commit -m "docs: document author-owned ordering model

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Notes for the implementer

- **Do not delete `impl Ord for CodeOwner`** in `src/structs.rs`. It is intentionally retained as the within-block fallback tiebreaker for team-derived entries, and its existing tests (`test_wildcard_sorts_before_regular_paths`, `test_negated_*`, and `test_file_path_sorting` in `src/tests/serialization.rs`) must stay green.
- **`author_order` must be captured before `expand_owner_groups`/`merge_teams_into_entries`** — the merge step returns `HashMap::into_values()` and destroys list order. `pipeline::process` already does this; don't move the capture.
- The renderer reproduces the existing per-line format exactly (column-aligned path, `!` negate prefix, leading-space owner accumulation), so the only output changes versus the old tool are: correct contiguous blocks, matching `END` footers, ungrouped-first, and alphabetical group order.
- `cargo test --lib` runs unit tests in `src/`; `cargo test` also runs the `src/tests/` modules. Use plain `cargo test` for the full sweep.
```
