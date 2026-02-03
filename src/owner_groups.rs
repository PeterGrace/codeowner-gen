use crate::structs::{CodeOwner, Owner, OwnerGroup, TeamPath};
use std::collections::HashMap;

/// Expands owner group references in entries and teams to their actual owners.
/// Returns an error if any owner group reference is not defined.
pub(crate) fn expand_owner_groups(
    owner_groups: &[OwnerGroup],
    entries: Vec<CodeOwner>,
    teams: HashMap<Owner, Vec<TeamPath>>,
) -> Result<(Vec<CodeOwner>, HashMap<Owner, Vec<TeamPath>>), String> {
    let group_map: HashMap<&str, &Vec<Owner>> = owner_groups
        .iter()
        .map(|g| (g.name.as_str(), &g.owners))
        .collect();

    let mut expanded_entries = Vec::new();

    for entry in entries {
        let expanded_owners = expand_owners(&entry.owners, &group_map)?;

        expanded_entries.push(CodeOwner {
            path: entry.path,
            owners: expanded_owners,
            comment: entry.comment,
            group: entry.group,
        });
    }

    // Expand teams
    let mut expanded_teams: HashMap<Owner, Vec<TeamPath>> = HashMap::new();
    for (owner, paths) in teams {
        match &owner {
            Owner::OwnerGroupRef(name) => {
                let group_owners = group_map
                    .get(name.as_str())
                    .ok_or_else(|| format!("Unknown owner group '{}' used as team key", name))?;

                for group_owner in *group_owners {
                    expanded_teams
                        .entry(group_owner.clone())
                        .or_insert_with(Vec::new)
                        .extend(paths.clone());
                }
            }
            _ => {
                expanded_teams
                    .entry(owner)
                    .or_insert_with(Vec::new)
                    .extend(paths);
            }
        }
    }

    Ok((expanded_entries, expanded_teams))
}

fn expand_owners(
    owners: &[Owner],
    group_map: &HashMap<&str, &Vec<Owner>>,
) -> Result<Vec<Owner>, String> {
    let mut expanded = Vec::new();

    for owner in owners {
        match owner {
            Owner::OwnerGroupRef(name) => {
                let group_owners = group_map.get(name.as_str()).ok_or_else(|| {
                    format!("Unknown owner group '{}' referenced in owners", name)
                })?;

                for group_owner in *group_owners {
                    if !expanded.contains(group_owner) {
                        expanded.push(group_owner.clone());
                    }
                }
            }
            _ => {
                if !expanded.contains(owner) {
                    expanded.push(owner.clone());
                }
            }
        }
    }

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_owner_group(name: &str, owners: Vec<Owner>) -> OwnerGroup {
        OwnerGroup {
            name: name.to_string(),
            owners,
        }
    }

    #[test]
    fn test_expand_owner_group_in_entries() {
        let owner_groups = vec![make_owner_group(
            "my_group",
            vec![
                Owner::Username("@alice".to_string()),
                Owner::Username("@bob".to_string()),
            ],
        )];

        let entries = vec![CodeOwner {
            path: "src/".to_string(),
            owners: vec![Owner::OwnerGroupRef("my_group".to_string())],
            comment: None,
            group: None,
        }];

        let (expanded_entries, _) =
            expand_owner_groups(&owner_groups, entries, HashMap::new()).unwrap();

        assert_eq!(expanded_entries.len(), 1);

        assert_eq!(expanded_entries[0].owners.len(), 2);

        assert!(
            expanded_entries[0]
                .owners
                .contains(&Owner::Username("@alice".to_string()))
        );

        assert!(
            expanded_entries[0]
                .owners
                .contains(&Owner::Username("@bob".to_string()))
        );
    }

    #[test]
    fn test_expand_owner_group_as_team_key() {
        let owner_groups = vec![make_owner_group(
            "my_group",
            vec![
                Owner::Username("@alice".to_string()),
                Owner::Team("@org/team".to_string()),
            ],
        )];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::OwnerGroupRef("my_group".to_string()),
            vec![TeamPath {
                path: "src/".to_string(),
                group: None,
            }],
        );

        let (_, expanded_teams) = expand_owner_groups(&owner_groups, vec![], teams).unwrap();

        assert_eq!(expanded_teams.len(), 2);
        assert!(expanded_teams.contains_key(&Owner::Username("@alice".to_string())));
        assert!(expanded_teams.contains_key(&Owner::Team("@org/team".to_string())));
    }

    #[test]
    fn test_unknown_owner_group_in_entries_fails() {
        let owner_groups = vec![];

        let entries = vec![CodeOwner {
            path: "src/".to_string(),
            owners: vec![Owner::OwnerGroupRef("nonexistent".to_string())],
            comment: None,
            group: None,
        }];

        let result = expand_owner_groups(&owner_groups, entries, HashMap::new());

        assert!(result.is_err());

        assert!(
            result
                .unwrap_err()
                .contains("Unknown owner group 'nonexistent'")
        );
    }

    #[test]
    fn test_unknown_owner_group_as_team_key_fails() {
        let owner_groups = vec![];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::OwnerGroupRef("nonexistent".to_string()),
            vec![TeamPath {
                path: "src/".to_string(),
                group: None,
            }],
        );

        let result = expand_owner_groups(&owner_groups, vec![], teams);

        assert!(result.is_err());

        assert!(
            result
                .unwrap_err()
                .contains("Unknown owner group 'nonexistent'")
        );
    }

    #[test]
    fn test_mixed_owners_and_group_refs() {
        let owner_groups = vec![make_owner_group(
            "my_group",
            vec![Owner::Username("@alice".to_string())],
        )];

        let entries = vec![CodeOwner {
            path: "src/".to_string(),
            owners: vec![
                Owner::Username("@bob".to_string()),
                Owner::OwnerGroupRef("my_group".to_string()),
            ],
            comment: None,
            group: None,
        }];

        let (expanded_entries, _) =
            expand_owner_groups(&owner_groups, entries, HashMap::new()).unwrap();

        assert_eq!(expanded_entries.len(), 1);

        assert_eq!(expanded_entries[0].owners.len(), 2);

        assert!(
            expanded_entries[0]
                .owners
                .contains(&Owner::Username("@bob".to_string()))
        );

        assert!(
            expanded_entries[0]
                .owners
                .contains(&Owner::Username("@alice".to_string()))
        );
    }

    #[test]
    fn test_deduplicates_owners() {
        let owner_groups = vec![make_owner_group(
            "my_group",
            vec![Owner::Username("@alice".to_string())],
        )];

        let entries = vec![CodeOwner {
            path: "src/".to_string(),
            owners: vec![
                Owner::Username("@alice".to_string()),
                Owner::OwnerGroupRef("my_group".to_string()),
            ],
            comment: None,
            group: None,
        }];

        let (expanded_entries, _) =
            expand_owner_groups(&owner_groups, entries, HashMap::new()).unwrap();

        assert_eq!(expanded_entries.len(), 1);
        assert_eq!(expanded_entries[0].owners.len(), 1);
    }

    #[test]
    fn test_preserves_non_group_owners() {
        let owner_groups = vec![];

        let entries = vec![CodeOwner {
            path: "src/".to_string(),
            owners: vec![
                Owner::Username("@alice".to_string()),
                Owner::Team("@org/team".to_string()),
                Owner::Email("test@example.com".to_string()),
            ],
            comment: None,
            group: None,
        }];

        let (expanded_entries, _) =
            expand_owner_groups(&owner_groups, entries, HashMap::new()).unwrap();

        assert_eq!(expanded_entries.len(), 1);
        assert_eq!(expanded_entries[0].owners.len(), 3);
    }
}
