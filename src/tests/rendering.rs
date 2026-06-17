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

// Entries authored so that author order (src/api/ before src/) differs from
// path order (src/ before src/api/), plus a team-only path in the same group.
// `{ALPHA}` is substituted with the desired alphabetize setting.
const TEMPLATE: &str = "
---
alphabetize: {ALPHA}
entries:
  - path: \"*\"
    owners: [\"@org/default\"]
  - path: \"src/api/\"
    group: \"core\"
    owners: [\"@org/api\"]
  - path: \"src/\"
    group: \"core\"
    owners: [\"@org/core\"]
  - path: \"docs/\"
    group: \"docs\"
    owners: [\"@org/writers\"]
teams:
  \"@org/legacy\":
    - path: \"src/legacy/\"
      group: \"core\"
";

fn render(alphabetize: bool) -> String {
    let input = TEMPLATE.replace("{ALPHA}", if alphabetize { "true" } else { "false" });
    let code_owners = serde_yaml::from_str::<CodeOwners>(&input).unwrap();
    let entries = process(code_owners).unwrap();
    render_body(&entries)
}

#[test]
fn test_block_order_is_independent_of_alphabetize() {
    // ungrouped first, then groups alphabetically by name (core before docs),
    // with matching BEGIN/END footers - true under both settings.
    for alphabetize in [true, false] {
        let body = render(alphabetize);
        assert_eq!(
            structure(&body),
            vec![
                "####### BEGIN UNGROUPED",
                "### END UNGROUPED",
                "####### BEGIN GROUP CORE",
                "### END GROUP CORE",
                "####### BEGIN GROUP DOCS",
                "### END GROUP DOCS",
            ],
            "alphabetize={}",
            alphabetize
        );
    }
}

#[test]
fn test_alphabetize_true_sorts_within_block_by_path() {
    let body = render(true);
    // core block sorted by path, overriding author order: src/, src/api/, src/legacy/
    assert!(line_index(&body, "src/") < line_index(&body, "src/api/"));
    assert!(line_index(&body, "src/api/") < line_index(&body, "src/legacy/"));
}

#[test]
fn test_alphabetize_false_preserves_author_order_then_team_fallback() {
    let body = render(false);
    // authored order is honored (src/api/ was written before src/)...
    assert!(line_index(&body, "src/api/") < line_index(&body, "src/"));
    // ...and the team-only path is appended after the authored entries.
    assert!(line_index(&body, "src/") < line_index(&body, "src/legacy/"));
}
