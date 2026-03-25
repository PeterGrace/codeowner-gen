use std::path::PathBuf;
use crate::structs::{CodeOwners, CodeOwner, Owner, OwnerGroup, TeamPath};
use std::collections::HashMap;
use serde_yaml;

fn team_path(s: &str) -> TeamPath {
    TeamPath {
        path: PathBuf::from(s),
        group: None,
    }
}

#[test]
fn test_single_username()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("*"),
                owners: vec![Owner::Username(String::from("@petergrace"))]
            }
        ],
        teams: HashMap::new(),
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_team()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"@my/team\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("*"),
                owners: vec![Owner::Team(String::from("@my/team"))]
            }
        ],
        teams: HashMap::new(),
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_email()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"pete.grace@gmail.com\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("*"),
                owners: vec![Owner::Email(String::from("pete.grace@gmail.com"))]
            }
        ],
        teams: HashMap::new(),
    };
    assert_eq!(value, control)
}

#[test]
fn test_multi_multi()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"foo/\"
    owners:
      - \"pete.grace@gmail.com\"
      - \"@my/team\"
  - path: \"bar/\"
    owners:
      - \"@petergrace\"
      - \"pete.grace@gmail.com\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("foo/"),
                owners: vec![
                    Owner::Email(String::from("pete.grace@gmail.com")),
                    Owner::Team(String::from("@my/team"))
                ]
            },
            CodeOwner{
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("bar/"),
                owners: vec![
                    Owner::Username(String::from("@petergrace")),
                    Owner::Email(String::from("pete.grace@gmail.com"))
                ]
            },

        ],
        teams: HashMap::new(),
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_with_comment_group()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"*\"
    comment: \"All the things\"
    group: \"primary\"
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner{
                comment: Some(String::from("All the things")),
                group: Some(String::from("primary")),
                negate: false,
                path: PathBuf::from("*"),
                owners: vec![Owner::Username(String::from("@petergrace"))]
            }
        ],
        teams: HashMap::new(),
    };

    assert_eq!(value, control)
}

#[test]
fn test_teams_only()
{
    const INPUT_DATA: &str = "
---
teams:
  \"@petergrace\":
    - \"alpha\"
    - \"zebra\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::Username(String::from("@petergrace")),
        vec![team_path("alpha"), team_path("zebra")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_teams_with_org_team()
{
    const INPUT_DATA: &str = "
---
teams:
  \"@myorg/team\":
    - \"src/\"
    - \"lib/\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::Team(String::from("@myorg/team")),
        vec![team_path("src/"), team_path("lib/")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_teams_with_email()
{
    const INPUT_DATA: &str = "
---
teams:
  \"pete.grace@gmail.com\":
    - \"docs/\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::Email(String::from("pete.grace@gmail.com")),
        vec![team_path("docs/")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_mixed_entries_and_teams()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"special/*\"
    comment: \"Needs metadata\"
    owners:
      - \"@admin\"
teams:
  \"@petergrace\":
    - \"src/\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::Username(String::from("@petergrace")),
        vec![team_path("src/")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner {
                comment: Some(String::from("Needs metadata")),
                group: None,
                negate: false,
                path: PathBuf::from("special/*"),
                owners: vec![Owner::Username(String::from("@admin"))]
            }
        ],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_invalid_owner_in_teams()
{
    // Invalid owner with special characters that aren't allowed
    const INPUT_DATA: &str = "
---
teams:
  \"invalid!owner\":
    - \"src/\"
";
    let result = serde_yaml::from_str::<CodeOwners>(INPUT_DATA);

    assert!(result.is_err());

    let err = result.unwrap_err().to_string();

    assert!(err.contains("Invalid owner format"), "Error was: {}", err);
}

#[test]
fn test_owner_group_ref_in_teams_is_parsed()
{
    const INPUT_DATA: &str = "
---
teams:
  \"my_team_group\":
    - \"src/\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::OwnerGroupRef(String::from("my_team_group")),
        vec![team_path("src/")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_empty_config()
{
    const INPUT_DATA: &str = "---";

    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![],
        teams: HashMap::new(),
    };

    assert_eq!(value, control)
}

#[test]
fn test_owner_groups_parsing()
{
    const INPUT_DATA: &str = "
---
owner_groups:
  - name: my_group
    owners:
      - \"@alice\"
      - \"@org/team\"
entries:
  - path: \"src/\"
    owners:
      - \"my_group\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![
            OwnerGroup {
                name: String::from("my_group"),
                owners: vec![
                    Owner::Username(String::from("@alice")),
                    Owner::Team(String::from("@org/team"))
                ]
            }
        ],
        entries: vec![
            CodeOwner {
                comment: None,
                group: None,
                negate: false,
                path: PathBuf::from("src/"),
                owners: vec![Owner::OwnerGroupRef(String::from("my_group"))]
            }
        ],
        teams: HashMap::new(),
    };

    assert_eq!(value, control)
}

#[test]
fn test_owner_groups_with_teams()
{
    const INPUT_DATA: &str = "
---
owner_groups:
  - name: bolt_team
    owners:
      - \"@bolt-ai\"
      - \"@bolt-core\"
teams:
  bolt_team:
    - \"/foo/\"
    - \"/bar/\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let mut expected_teams = HashMap::new();

    expected_teams.insert(
        Owner::OwnerGroupRef(String::from("bolt_team")),
        vec![team_path("/foo/"), team_path("/bar/")]
    );

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![
            OwnerGroup {
                name: String::from("bolt_team"),
                owners: vec![
                    Owner::Username(String::from("@bolt-ai")),
                    Owner::Username(String::from("@bolt-core"))
                ]
            }
        ],
        entries: vec![],
        teams: expected_teams,
    };

    assert_eq!(value, control)
}

#[test]
fn test_nested_owner_group_refs_not_allowed()
{
    const INPUT_DATA: &str = "
---
owner_groups:
  - name: group_a
    owners:
      - \"group_b\"
";
    let result = serde_yaml::from_str::<CodeOwners>(INPUT_DATA);

    assert!(result.is_err());
}

#[test]
fn test_file_path_sorting()
{
    const INPUT_DATA: &str = "
---
teams:
  \"@owner\":
    - \"path-to-a-file\"
    - \"path/\"
    - \"another-path-to-a-file\"
    - \"path/to-a-nested-dir/\"
  \"team\":
    - \"*\"
    - \"path/*.js\"
    - \"path/to-a-file-in-a-dir\"
";
    let code_owners = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    let mut entries = crate::teams::merge_teams_into_entries(code_owners.entries, code_owners.teams);
    entries.sort();
    let paths: Vec<&str> = entries.iter().map(|e| e.path.to_str().unwrap()).collect();

    assert_eq!(paths, vec![
        "*",
        "another-path-to-a-file",
        "path/",
        "path/*.js",
        "path/to-a-file-in-a-dir",
        "path/to-a-nested-dir/",
        "path-to-a-file",
    ]);
}

#[test]
fn test_negated_entry_parsed()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"docs/generated/\"
    negate: true
    owners: []
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    assert!(value.entries[0].negate);
    assert_eq!(value.entries[0].path, PathBuf::from("docs/generated/"));
    assert!(value.entries[0].owners.is_empty());
}

#[test]
fn test_negate_defaults_to_false()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"src/\"
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();
    assert!(!value.entries[0].negate);
}

#[test]
fn test_negated_entry_with_owners()
{
    const INPUT_DATA: &str = "
---
entries:
  - path: \"docs/\"
    negate: true
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let control: CodeOwners = CodeOwners {
        owner_groups: vec![],
        entries: vec![
            CodeOwner {
                comment: None,
                group: None,
                negate: true,
                path: PathBuf::from("docs/"),
                owners: vec![Owner::Username(String::from("@petergrace"))]
            }
        ],
        teams: HashMap::new(),
    };

    assert_eq!(value, control);
}
