use lazy_static::lazy_static;
use regex::Regex;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

lazy_static! {
    static ref TEAM: Regex = Regex::new(r"^@\S+/\S+").unwrap();
    static ref USERNAME: Regex = Regex::new(r"^@\S+").unwrap();
    static ref EMAIL: Regex = Regex::new(r"^\S+@\S+").unwrap();
}

/// Defines a group of owners that can be referenced by name
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct OwnerGroup {
    pub(crate) name: String,
    /// Owners in this group (must be valid owners, not other group refs)
    #[serde(deserialize_with = "strict_owners_from_string")]
    pub(crate) owners: Vec<Owner>,
}

#[derive(Deserialize, Default, Debug, PartialEq)]
pub(crate) struct CodeOwners {
    /// a list of owner group definitions that can be referenced by name
    #[serde(default)]
    pub(crate) owner_groups: Vec<OwnerGroup>,

    /// a list of CodeOwner entries, that resolve to a line in CODEOWNERS file.
    #[serde(default)]
    pub(crate) entries: Vec<CodeOwner>,

    /// a mapping of owner to list of paths they own
    #[serde(default)]
    pub(crate) teams: HashMap<Owner, Vec<TeamPath>>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TeamPath {
    pub(crate) path: PathBuf,
    pub(crate) group: Option<String>,
}

impl<'de> Deserialize<'de> for TeamPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TeamPathVisitor;

        impl<'de> Visitor<'de> for TeamPathVisitor {
            type Value = TeamPath;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string path or an object with 'path' and optional 'group'")
            }

            fn visit_str<E>(self, value: &str) -> Result<TeamPath, E>
            where
                E: Error,
            {
                Ok(TeamPath {
                    path: PathBuf::from(value),
                    group: None,
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<TeamPath, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut path: Option<PathBuf> = None;
                let mut group: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "path" => {
                            if path.is_some() {
                                return Err(Error::duplicate_field("path"));
                            }
                            path = Some(map.next_value()?);
                        }
                        "group" => {
                            if group.is_some() {
                                return Err(Error::duplicate_field("group"));
                            }
                            group = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(Error::unknown_field(&key, &["path", "group"]));
                        }
                    }
                }

                let path = path.ok_or_else(|| Error::missing_field("path"))?;
                Ok(TeamPath { path, group })
            }
        }

        deserializer.deserialize_any(TeamPathVisitor)
    }
}

#[derive(Deserialize, Default, Debug, PartialEq, Eq)]
pub(crate) struct CodeOwner {
    pub(crate) path: PathBuf,
    #[serde(default)]
    pub(crate) negate: bool,
    #[serde(deserialize_with = "owners_from_string")]
    pub(crate) owners: Vec<Owner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<String>,
}

impl PartialOrd for CodeOwner {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// File-path sorting
// We need to use custom sorting to ensure * always comes before other entries.
// Paths like /path/*.js will also correctly be ordered first, but due to ASCII ordering.
impl Ord for CodeOwner {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let a_str = self.path.to_str().unwrap_or("");
        let b_str = other.path.to_str().unwrap_or("");
        match (a_str.starts_with('*'), b_str.starts_with('*')) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => self.path.cmp(&other.path)
                .then(self.negate.cmp(&other.negate)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> CodeOwner {
        CodeOwner {
            path: PathBuf::from(path),
            negate: false,
            owners: vec![],
            comment: None,
            group: None,
        }
    }

    fn negated_entry(path: &str) -> CodeOwner {
        CodeOwner {
            path: PathBuf::from(path),
            negate: true,
            owners: vec![],
            comment: None,
            group: None,
        }
    }

    #[test]
    fn test_wildcard_sorts_before_regular_paths() {
        let mut entries = [entry("src/"), entry("*"), entry("lib/")];
        entries.sort();
        assert_eq!(entries[0].path, PathBuf::from("*"));
    }

    #[test]
    fn test_wildcard_prefix_sorts_before_regular_paths() {
        let mut entries = [entry("src/"), entry("*.rs"), entry("lib/")];
        entries.sort();
        assert_eq!(entries[0].path, PathBuf::from("*.rs"));
    }

    #[test]
    fn test_wildcard_prefix_sorts_before_regular_paths_in_dir() {
        let mut entries = [entry("src/*.rs"), entry("src/README.md")];
        entries.sort();
        assert_eq!(entries[0].path, PathBuf::from("src/*.rs"));
    }

    #[test]
    fn test_directory_sorts_before_hyphenated_path_at_same_prefix() {
        let mut entries = [entry("path-to-a-file"), entry("path/")];
        entries.sort();
        assert_eq!(entries[0].path, PathBuf::from("path/"));
    }

    #[test]
    fn test_negated_entry_sorts_after_same_path() {
        let mut entries = [negated_entry("docs/"), entry("docs/")];
        entries.sort();
        assert!(!entries[0].negate);
        assert!(entries[1].negate);
    }

    #[test]
    fn test_negated_subpath_sorts_after_parent() {
        let mut entries = [negated_entry("docs/generated/"), entry("docs/")];
        entries.sort();
        assert_eq!(entries[0].path, PathBuf::from("docs/"));
        assert_eq!(entries[1].path, PathBuf::from("docs/generated/"));
    }
}

fn owners_from_string<'de, D>(input: D) -> Result<Vec<Owner>, D::Error>
where
    D: Deserializer<'de>,
{
    owners_from_string_internal(input, true)
}

fn strict_owners_from_string<'de, D>(input: D) -> Result<Vec<Owner>, D::Error>
where
    D: Deserializer<'de>,
{
    owners_from_string_internal(input, false)
}

fn owners_from_string_internal<'de, D>(input: D, allow_group_refs: bool) -> Result<Vec<Owner>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut results: Vec<Owner> = vec![];
    let input_strings: Vec<String> = Vec::deserialize(input)?;

    for inpstr in input_strings {
        for obj in inpstr.split_whitespace() {
            if TEAM.is_match(obj) {
                results.push(Owner::Team(obj.to_string()))
            } else if USERNAME.is_match(obj) {
                results.push(Owner::Username(obj.to_string()))
            } else if EMAIL.is_match(obj) {
                results.push(Owner::Email(obj.to_string()))
            } else if allow_group_refs && is_valid_group_name(obj) {
                results.push(Owner::OwnerGroupRef(obj.to_string()))
            } else {
                return Err(Error::custom(
                    "Invalid value for owner. Expected @username, @team/name, email@email.com, or owner_group name",
                ));
            }
        }
    }
    Ok(results)
}

/// Check if a string is a valid owner group name (alphanumeric, underscores, hyphens)
fn is_valid_group_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum Owner {
    /// Owner in the form @username
    Username(String),
    /// Owner in the form @org/Team
    Team(String),
    /// Owner in the form user@domain.com
    Email(String),
    /// Reference to an owner group by name (bare string without @)
    OwnerGroupRef(String),
}

impl FromStr for Owner {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if TEAM.is_match(s) {
            Ok(Owner::Team(s.to_string()))
        } else if USERNAME.is_match(s) {
            Ok(Owner::Username(s.to_string()))
        } else if EMAIL.is_match(s) {
            Ok(Owner::Email(s.to_string()))
        } else if is_valid_group_name(s) {
            Ok(Owner::OwnerGroupRef(s.to_string()))
        } else {
            Err("Invalid owner format. Expected @username, @org/team, email@domain.com, or owner_group name".into())
        }
    }
}

impl std::fmt::Display for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Owner::Email(v) | Owner::Team(v) | Owner::Username(v) | Owner::OwnerGroupRef(v) => v.fmt(f),
        }
    }
}

impl<'de> Deserialize<'de> for Owner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OwnerVisitor;

        impl<'de> Visitor<'de> for OwnerVisitor {
            type Value = Owner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string like @username, @org/team, email@domain.com, or owner_group name")
            }

            fn visit_str<E>(self, value: &str) -> Result<Owner, E>
            where
                E: Error,
            {
                Owner::from_str(value).map_err(Error::custom)
            }
        }

        deserializer.deserialize_str(OwnerVisitor)
    }
}
