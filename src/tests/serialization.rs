use crate::structs::{CodeOwners, CodeOwner, Owner, TeamPath};
use std::collections::HashMap;
use serde_yaml;

fn team_path(s: &str) -> TeamPath {
    TeamPath {
        path: s.to_string(),
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
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
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
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
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
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
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
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("foo/"),
                owners: vec![
                    Owner::Email(String::from("pete.grace@gmail.com")),
                    Owner::Team(String::from("@my/team"))
                ]
            },
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("bar/"),
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
        entries: vec![
            CodeOwner{
                comment: Some(String::from("All the things")),
                group: Some(String::from("primary")),
                path: String::from("*"),
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
        entries: vec![
            CodeOwner {
                comment: Some(String::from("Needs metadata")),
                group: None,
                path: String::from("special/*"),
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
    const INPUT_DATA: &str = "
---
teams:
  \"invalid-owner\":
    - \"src/\"
";
    let result = serde_yaml::from_str::<CodeOwners>(INPUT_DATA);

    assert!(result.is_err());

    let err = result.unwrap_err().to_string();

    assert!(err.contains("Invalid owner format"), "Error was: {}", err);
}

#[test]
fn test_empty_config()
{
    const INPUT_DATA: &str = "---";

    let value = serde_yaml::from_str::<CodeOwners>(INPUT_DATA).unwrap();

    let control: CodeOwners = CodeOwners {
        entries: vec![],
        teams: HashMap::new(),
    };

    assert_eq!(value, control)
}