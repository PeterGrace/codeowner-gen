# codeowner-gen

## GitHub Actions users, read this!
This action will pull the specified version of codeowner-gen and execute it against your codeowners.yaml file, outputting a CODEOWNERS file at the path you specify.  You can have the script commit back the change into the repo automatically.

To use this tool in your repository, add this stanza to your github actions, changing the arguments as needed:
```yaml
      - name: Run codeowner-gen
        uses: PeterGrace/codeowner-gen@latest
        with:
          config-file: <path-to-codeowners.yaml>
          output-file: <path-to-expected-CODEOWNERS-file>
          commit: [true|false]  ### optional
          commit-message: 'your commit message'  ### optional
          version: [latest|0.4.2]  ### optional
```

## Background

When I wrote this project originally, I worked for a company that has a staff with an attention to well-formatted, pretty files. If a change is made to a CODEOWNERS file that would require the file to be re-columned, then the PR would show all lines changed. This would prompt a PR reviewer to have to either click "approve" without considering the content of the file, or actually read the entire CODEOWNERS file again. So, I wrote codeowner-gen with this problem in mind.

codeowner-gen takes a well-formatted yaml file, and does a few things:

- You specify a bunch of paths and one or more owners per path, and it will ensure that all records are spaced so that output is columnar,
- You specify one or more `owner_groups` which is a group of people or teams, and they can then be assigned to paths,
- entries are emitted in the order you wrote them (see [Ordering and grouping](#ordering-and-grouping) below),
- entries can optionally be assigned to named groups, which controls the block order in the output,
- it processes the Owners you've listed and I might eventually enable the app to validate that the user/team you've specified actually exists,
- You can specify a comment, per path entry, and it will render it out for you.

The output is then rendered to the CODEOWNERS file for you, grouped and ordered by your specification, and properly columned so that the text columns align.

With this workflow, a reviewer can see that the first line of the CODEOWNERS file is a codeowner-gen rendered file and ignore it, in favor of reviewing the changes in the codeowners.yaml file instead. That file, being yaml, will show changes in a more sane and easy-to-digest format for a PR reviewer.

## How-to install (Download precompiled binary)

The latest release version is available in the Releases area of GitHub, with Linux and Windows binaries pre-created.  Download from there.

## How-to install (build locally)

`cargo install --git https://github.com/PeterGrace/codeowner-gen.git`

## Usage

`codeowner-gen` in a directory with a well-formatted codeowners.yaml will output a CODEOWNERS file. If you want to specify an alternate yaml, use `-i` option.

### Entries Format

The traditional format uses an `entries` array where each entry specifies a path and its owners:

```yaml
---
entries:
  - path: "alpha"
    comment: "Alphabetically speaking, this probably is coming early on"
    group: "phonetic"
    owners:
      - "@petergrace"
  - path: "zebra"
    comment: "This is likely the last entry"
    group: "phonetic"
    owners:
      - "@petergrace"
  - path: "*"
    group: "main"
    owners:
      - "@petergrace"
  - path: "target/debug/deps/itoa-*"
    owners:
      - "pete.grace@gmail.com"
      - "@petergrace"
      - "@petergrace/teamname"
```

### Teams Format

Alternatively, you can use the `teams` format which maps owners to a list of paths they own. This is useful when you want to manage ownership by team rather than by path:

```yaml
---
teams:
  "@petergrace":
    - "alpha"
    - "zebra"
  "@myorg/platform-team":
    - "src/"
    - "lib/"
  "pete.grace@gmail.com":
    - "docs/"
```

### Teams Format with Groups

Paths in the teams format can also include a `group` field. Use an object with `path` and `group` instead of a simple string:

```yaml
---
teams:
  "@petergrace":
    - path: "src/"
      group: "core"
    - "lib/"
  "@myorg/platform-team":
    - path: "infra/"
      group: "infrastructure"
```

### Owner Groups

Owner groups let you define reusable collections of owners that can be referenced by name. This is useful when the same set of owners appears in multiple places:

```yaml
---
owner_groups:
  - name: platform_team
    owners:
      - "@alice"
      - "@bob"
      - "@myorg/platform"
  - name: security_team
    owners:
      - "@security-lead"
      - "@myorg/security"
```

Once defined, owner groups can be referenced by their name (without `@`) in both `entries` and `teams`:

```yaml
---
owner_groups:
  - name: platform_team
    owners:
      - "@alice"
      - "@bob"

entries:
  - path: "src/"
    owners:
      - "platform_team"
      - "@extra-reviewer"

teams:
  platform_team:
    - "lib/"
    - "infra/"
```

In this example:

- `src/` will have owners `@alice @bob @extra-reviewer`
- `lib/` and `infra/` will each have owners `@alice @bob`

**Note:** Owner groups cannot reference other owner groups (no nesting). The owners within an owner group must be valid GitHub usernames (`@user`), teams (`@org/team`), or email addresses.

### Negated Entries

Setting `negate: true` on an entry causes the path to be rendered with a leading `!` in the CODEOWNERS file. This is the standard GitHub CODEOWNERS syntax for removing a path from a previously-matched ownership rule.

```yaml
---
entries:
  - path: "docs/"
    owners:
      - "@petergrace"
  - path: "docs/generated/"
    negate: true
    owners: []
```

This produces output like:

```
docs/           @petergrace
!docs/generated/
```

GitHub interprets the `!` prefix as "no owner for this path", overriding the `docs/` rule for the `docs/generated/` subtree. `negate` defaults to `false` and is only needed when you want to explicitly exclude a path from ownership.

### Mixed Format

You can combine both formats. When the same path appears in both `teams` and `entries`, owners are merged and the entry's metadata (comment/group) is preserved:

```yaml
---
teams:
  "@myorg/team":
    - "src/"
entries:
  - path: "src/"
    comment: "Core source code"
    owners:
      - "@admin"
```

This results in `src/` having owners `@myorg/team @admin` with the comment "Core source code".

**Note:** If the same path has a `group` defined in both `entries` and `teams`, the entry's group takes precedence. The team's group is only applied if the entry doesn't already have a group.

### Ordering and grouping

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

### Example Output

Given a yaml file like below:

```yaml
---
entries:
  - path: "alpha"
    comment: "Alphabetically speaking, this probably is coming early on"
    group: "phonetic"
    owners:
      - "@petergrace"
  - path: "zebra"
    comment: "This is likely the last entry"
    group: "phonetic"
    owners:
      - "@petergrace"
  - path: "*"
    group: "main"
    owners:
      - "@petergrace"
  - path: "target/debug/deps/itoa-*"
    owners:
      - "pete.grace@gmail.com"
      - "@petergrace"
      - "@petergrace/teamname"
```

The codeowners-gen program will output:

```
# Generated by codeowner-gen v0.1.0/2db3d0dbd7ece3182de63b440b07379cb19b49cd
#
####### BEGIN UNGROUPED
target/debug/deps/itoa-*  pete.grace@gmail.com @petergrace @petergrace/teamname
####### BEGIN GROUP MAIN
*                         @petergrace
### END GROUP MAIN
####### BEGIN GROUP PHONETIC
# Alphabetically speaking, this probably is coming early on
alpha                     @petergrace
# This is likely the last entry
zebra                     @petergrace
```
