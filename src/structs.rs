use serde::de::{Error};
use serde::{Deserialize, Deserializer};
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
            static ref TEAM: Regex = Regex::new(r"^@\S+/\S+").unwrap();
            static ref USERNAME: Regex = Regex::new(r"^@\S+").unwrap();
            static ref EMAIL: Regex = Regex::new(r"^\S+@\S+").unwrap();
}

#[derive(Deserialize, Default, Debug, PartialEq)]
pub(crate) struct CodeOwners {
    /// a list of CodeOwner entries, that resolve to a line in CODEOWNERS file.
    pub(crate) entries: Vec<CodeOwner>
}

#[derive(Deserialize, Default, Debug, PartialEq)]
pub(crate) struct CodeOwner {
    pub(crate) path: String,
    #[serde(deserialize_with = "owners_from_string")]
    pub(crate) owners: Vec<Owner>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub(crate) group: Option<String>
}

fn owners_from_string<'de, D>(input: D) -> Result<Vec<Owner>, D::Error>
where D: Deserializer<'de> {
    let mut results: Vec<Owner> = vec![];
    let input_strings :Vec<String> = Vec::deserialize(input)?;
    for inpstr in input_strings {
    for obj in inpstr.split_whitespace() {
        if TEAM.is_match(obj) {
            results.push(Owner::Team(obj.to_string()))
        } else if USERNAME.is_match(obj) {
            results.push(Owner::Username(obj.to_string()))
        } else if EMAIL.is_match(obj) {
            results.push(Owner::Email(obj.to_string()))
        } else {
            return Err(Error::custom("invalid value for owner.  Expected @username, @team/name, or email@email.com"));
        }

    }
    }
    Ok(results)
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum Owner {
    /// Owner in the form @username
    Username(String),
    /// Owner in the form @org/Team
    Team(String),
    /// Owner in the form user@domain.com
    Email(String),
}
impl std::fmt::Display for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            // v is a String in both cases
            Owner::Email(v) | Owner::Team(v) | Owner::Username(v) => v.fmt(f),
        }
    }
}