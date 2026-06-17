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
