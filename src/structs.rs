use lazy_static::lazy_static;
use regex::Regex;
use serde::de::{Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::str::FromStr;

lazy_static! {
    static ref TEAM: Regex = Regex::new(r"^@\S+/\S+").unwrap();
    static ref USERNAME: Regex = Regex::new(r"^@\S+").unwrap();
    static ref EMAIL: Regex = Regex::new(r"^\S+@\S+").unwrap();
}

#[derive(Deserialize, Default, Debug, PartialEq)]
pub(crate) struct CodeOwners {
    /// A list of CodeOwner entries, that resolve to a line in CODEOWNERS file
    #[serde(default)]
    pub(crate) entries: Vec<CodeOwner>,

    /// A mapping of owner(s) to list of paths they own
    #[serde(default)]
    pub(crate) teams: HashMap<OwnerKey, Vec<TeamPath>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct OwnerKey(pub(crate) Vec<Owner>);

impl<'de> Deserialize<'de> for OwnerKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OwnerKeyVisitor;

        impl<'de> Visitor<'de> for OwnerKeyVisitor {
            type Value = OwnerKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "A single owner string like '@alice' or an array like ['@alice', '@bob']",
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<OwnerKey, E>
            where
                E: Error,
            {
                let owner = Owner::from_str(value).map_err(Error::custom)?;
                Ok(OwnerKey(vec![owner]))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<OwnerKey, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut owners = Vec::new();

                while let Some(value) = seq.next_element::<String>()? {
                    let owner = Owner::from_str(&value).map_err(Error::custom)?;
                    owners.push(owner);
                }

                if owners.is_empty() {
                    return Err(Error::custom("Owner list cannot be empty"));
                }

                Ok(OwnerKey(owners))
            }
        }

        deserializer.deserialize_any(OwnerKeyVisitor)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct TeamPath {
    pub(crate) path: String,
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
                    path: value.to_string(),
                    group: None,
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<TeamPath, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut path: Option<String> = None;
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

#[derive(Deserialize, Default, Debug, PartialEq)]
pub(crate) struct CodeOwner {
    pub(crate) path: String,
    #[serde(deserialize_with = "owners_from_string")]
    pub(crate) owners: Vec<Owner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<String>,
}

fn owners_from_string<'de, D>(input: D) -> Result<Vec<Owner>, D::Error>
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
            } else {
                return Err(Error::custom(
                    "Invalid value for owner. Expected @username, @team/name, or email@email.com",
                ));
            }
        }
    }
    Ok(results)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Owner {
    /// Owner in the form @username
    Username(String),
    /// Owner in the form @org/Team
    Team(String),
    /// Owner in the form user@domain.com
    Email(String),
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
        } else {
            Err("Invalid owner format. Expected @username, @org/team, or email@domain.com".into())
        }
    }
}

impl std::fmt::Display for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            // v is a String in both cases
            Owner::Email(v) | Owner::Team(v) | Owner::Username(v) => v.fmt(f),
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
                formatter.write_str("a string like @username, @org/team, or email@domain.com")
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
