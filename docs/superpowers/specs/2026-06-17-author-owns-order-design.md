# Design: "Author owns order" + uniform grouping

Date: 2026-06-17

## Background

`codeowner-gen` renders a GitHub `CODEOWNERS` file from a YAML spec. GitHub
resolves ownership by **last matching pattern wins** — position in the file is
the entire precedence model; specificity is irrelevant.

The current implementation does not honor this. It sorts the entry list twice
(`main.rs`): first by group label, then by a path-based `Ord`
(`structs.rs`). The second sort overwrites the first, which:

- **Breaks grouping.** Entries are interleaved by path, so group blocks are no
  longer contiguous; the output loop emits repeated/again headers, leaks
  ungrouped entries into group blocks, and never closes the final block.
- **Silently rewrites precedence.** Because order *is* meaning in a
  last-match-wins format, sorting by path is a semantic edit. The path `Ord`
  (with its forced `*`-first rule) is a heuristic approximating
  "most-specific-wins" — a model GitHub does not actually use. The author has no
  way to express the order they want.

This design replaces the implicit, heuristic ordering with an explicit,
author-controlled model.

## Core principle

> **The author owns order. The tool reorders only when explicitly asked. The
> default is to preserve the order as written.**

The tool's job is to *resolve, merge, align, validate, and emit* — never to
silently second-guess precedence.

## The render model

The renderable universe, after alias expansion and team-merging, is a single
flat list of `CodeOwner` entries. The model is one sentence:

> Partition entries by group; emit `ungrouped` first, then every other group
> alphabetically by name; within each block, keep author order (team-derived
> lines with no authored position fall back to alphabetical-by-path, after the
> authored lines).

### 1. Every entry has a group

`group` parses as `Option<String>` but is normalized to `"ungrouped"`
immediately after parse/merge. The rest of the pipeline only ever sees real
group names — no `None`/`Some` special-casing in the sort or renderer.

`ungrouped` is a **reserved** group name. If an author writes
`group: ungrouped` explicitly, those entries simply join the ungrouped block.
This is documented behavior, not an error.

### 2. Block order

- The `ungrouped` block is emitted **first**. Rationale: in last-match-wins,
  first = weakest precedence, so un-curated/default entries (often a bare `*`
  catch-all) form a low-precedence baseline that the deliberately-labeled group
  sections below can override. It also matches where the original tool placed
  the ungrouped block.
- All other group blocks follow, sorted **alphabetically by group name**.

Alphabetical block ordering is a deliberate, accepted exception to "author owns
order" at the *macro* level: section arrangement is deterministic and
predictable rather than hand-maintained. Because a group's position is decided
by its **name**, block placement is fully deterministic regardless of whether
the group first appeared via `entries:` (ordered) or `teams:` (an unordered
map).

Cross-block precedence consequence: because blocks are concatenated, a block
later in the file (alphabetically later name) has higher precedence than an
earlier one. The author accepts this in exchange for deterministic section
ordering. Fine-grained, precedence-sensitive ordering is controlled *within* a
block, which stays author-owned.

### 3. Within a block

- Entries that have an **authored position** (they appeared in the `entries:`
  list) render in that author order.
- Entries that exist **only because of the `teams:` mapping** have no authored
  position (`teams:` is a YAML map, inherently unordered). These fall back to
  **alphabetical by path** and are clustered **after** the authored lines in the
  block.

Sort key within a block: `(author_index, path)` where `author_index` is the
entry's index in the original `entries:` list, or "after all authored entries"
when it came only from `teams:`. The path tiebreaker reuses the existing
`CodeOwner` `Ord` (see below).

## Data flow (unchanged stages in *italics*, changed in **bold**)

1. *Parse YAML* → `owner_groups` + `entries` + `teams`.
2. **Capture author order.** Before merging, record each entry's path → its
   index in the original `entries:` list (`HashMap<PathBuf, usize>`). This is
   the only stable source of authored order; the merge step destroys list order
   (`teams.rs` returns `HashMap::into_values()`).
3. *Expand `owner_groups` aliases* → real owners, in entries and team keys.
4. *Merge `teams:` into entries* by path (dedup owners). Output order remains
   non-deterministic — that is now acceptable because the sort key does not
   depend on post-merge position.
5. **Normalize groups:** every entry's `group` becomes `Some(name)` →
   `"ungrouped"` when absent.
6. **Sort** with the composite key:
   `(group == "ungrouped" ? 0 : 1, group_name, author_index_or_max, path Ord)`.
   - ungrouped block first (`0` before `1`),
   - then alphabetical by group name,
   - then authored order, then path fallback.
7. **Render** (see below).

## Removed / repurposed behavior

- **Removed:** the whole-file path sort (`code_owners.entries.sort()` in
  `main.rs`) and the separate group sort (`sort_by_key`). Replaced by the single
  composite sort.
- **Removed:** auto-`*`-first as a precedence decider. The author is now
  responsible for catch-all placement. (Optional future enhancement: a
  *warning* when a `*` shadows rules, but the tool never *moves* it.)
- **Repurposed, not deleted:** the `CodeOwner` `Ord` impl (`structs.rs`) is
  demoted from "the whole-file order" to "the within-block path tiebreaker for
  team-derived entries with no authored position." Its existing `*`-first +
  negate-tiebreak behavior is fine for this deterministic-fallback role, and its
  existing tests continue to pass unchanged.

## Rendering

Extract rendering out of `main()` into a pure, testable function (proposed:
`src/render.rs`, `render_body(entries: &[CodeOwner]) -> String`). It walks the
already-sorted entries and tracks the current block label:

- On label change: close the previous block (`### END <LABEL>`) if one is open,
  then open the new block (`####### BEGIN <LABEL>`).
- After the loop: close the final open block.

Because the list is correctly sorted into contiguous blocks first, headers never
repeat and every opened block gets a matching footer (fixing the current missing
`END` footers). Blocks are entirely entry-driven, so declared-but-unused groups
never appear.

Per-line formatting is preserved: column-aligned path (width = longest path,
accounting for the `!` negate prefix), `!`-prefix for `negate`, a `# comment`
line above the entry when present, and space-joined owners.

`main.rs` slims to: parse → capture author order → expand → merge → normalize →
sort → write version header → write `render_body(...)`. The `grouped` boolean
flag is removed.

## Example

Input (`entries:` interleaved on purpose; one team-only path):

```yaml
entries:
  - path: "*"           # ungrouped
    owners: ["@org/default"]
  - path: "src/"
    group: "core"
    owners: ["@org/core"]
  - path: "docs/"
    group: "docs"
    owners: ["@org/writers"]
  - path: "src/api/"
    group: "core"
    owners: ["@org/api"]
teams:
  "@org/legacy":
    - path: "src/legacy/"   # only source for this path -> team-derived
      group: "core"
```

Output:

```
####### BEGIN UNGROUPED
*           @org/default
### END UNGROUPED
####### BEGIN GROUP CORE
src/        @org/core          # authored order...
src/api/    @org/api
src/legacy/ @org/legacy        # team-derived, alphabetical-by-path, after authored
### END GROUP CORE
####### BEGIN GROUP DOCS
docs/       @org/writers
### END GROUP DOCS
```

(`ungrouped` first; `core` before `docs` alphabetically; within `core`, the two
authored lines keep author order and the team-only `src/legacy/` is appended.)

## Testing

- **Sort (in `structs.rs` or a dedicated module):**
  - `ungrouped` block sorts first.
  - Non-ungrouped groups sort alphabetically by name.
  - Within a block, authored entries keep author order.
  - Team-derived entries (no authored index) fall to alphabetical-by-path after
    authored entries.
  - Existing path-`Ord` tests remain green (Ord unchanged).
- **Render (in `render.rs`):** a mixed fixture (multiple groups, ungrouped,
  negate, comment, team-derived) asserts the exact output string — proving
  contiguous blocks, matching `BEGIN`/`END` footers, ungrouped-first, and
  alphabetical block order.
- **End-to-end:** update `codeowners.yaml` sample and any golden expectations to
  the new layout.

## Out of scope (explicitly not included)

- **Per-block "alphabetize the entries within this block" opt-in.** Earlier in
  discussion a within-block alphabetical toggle was floated. It is *not* included
  here: within-block order is author-owned, with alphabetical used only as the
  deterministic fallback for team-derived lines. Flagged so it can be added later
  if a real need appears.
- **A numeric `priority` field.** The original request ("an entry that ends the
  file no matter what") is satisfied structurally: the author places that entry
  last within its block, and block order is deterministic. No priority field is
  introduced.
- **Shadowing/`*`-placement warnings.** Possible future enhancement; not part of
  this change.

## Documentation

`README.md` must be updated to describe: author-owned ordering, the implicit
`ungrouped` group, `ungrouped`-first + alphabetical block ordering, within-block
author order, and the team-derived alphabetical fallback.
