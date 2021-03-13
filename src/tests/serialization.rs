use crate::lib::{CodeOwners, CodeOwner, Owner};
use serde::Deserialize;
use serde_yaml;

#[test]
fn test_single_username()
{
    const input_data: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(input_data).unwrap();
    let control: CodeOwners = CodeOwners {
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
                owners: vec![Owner::Username(String::from("@petergrace"))]
            }
        ]
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_team()
{
    const input_data: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"@my/team\"
";
    let value = serde_yaml::from_str::<CodeOwners>(input_data).unwrap();
    let control: CodeOwners = CodeOwners {
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
                owners: vec![Owner::Team(String::from("@my/team"))]
            }
        ]
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_email()
{
    const input_data: &str = "
---
entries:
  - path: \"*\"
    owners:
      - \"pete.grace@gmail.com\"
";
    let value = serde_yaml::from_str::<CodeOwners>(input_data).unwrap();
    let control: CodeOwners = CodeOwners {
        entries: vec![
            CodeOwner{
                comment: None,
                group: None,
                path: String::from("*"),
                owners: vec![Owner::Email(String::from("pete.grace@gmail.com"))]
            }
        ]
    };
    assert_eq!(value, control)
}

#[test]
fn test_multi_multi()
{
    const input_data: &str = "
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
    let value = serde_yaml::from_str::<CodeOwners>(input_data).unwrap();
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

        ]
    };
    assert_eq!(value, control)
}

#[test]
fn test_single_with_comment_group()
{
    const input_data: &str = "
---
entries:
  - path: \"*\"
    comment: \"All the things\"
    group: \"primary\"
    owners:
      - \"@petergrace\"
";
    let value = serde_yaml::from_str::<CodeOwners>(input_data).unwrap();
    let control: CodeOwners = CodeOwners {
        entries: vec![
            CodeOwner{
                comment: Some(String::from("All the things")),
                group: Some(String::from("primary")),
                path: String::from("*"),
                owners: vec![Owner::Username(String::from("@petergrace"))]
            }
        ]
    };
    assert_eq!(value, control)
}