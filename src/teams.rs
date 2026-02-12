use crate::structs::{CodeOwner, Owner, TeamPath};
use std::collections::HashMap;

/// Merges teams mapping into entries, combining owners for duplicate paths.
/// When the same path appears in both teams and entries, owners are merged
/// and the entry's metadata (comment/group) is preserved. Groups from teams
/// are also applied if the entry doesn't already have a group.
/// Owners are ordered alphabetically by type, with users first, then teams.
pub(crate) fn merge_teams_into_entries(
    entries: Vec<CodeOwner>,
    teams: HashMap<Owner, Vec<TeamPath>>,
) -> Vec<CodeOwner> {
    let mut path_map: HashMap<String, CodeOwner> = HashMap::new();

    for entry in entries {
        path_map
            .entry(entry.path.clone())
            .and_modify(|existing| {
                for owner in &entry.owners {
                    if !existing.owners.contains(owner) {
                        existing.owners.push(owner.clone());
                    }
                }

                if existing.comment.is_none() && entry.comment.is_some() {
                    existing.comment = entry.comment.clone();
                }

                if existing.group.is_none() && entry.group.is_some() {
                    existing.group = entry.group.clone();
                }
            })
            .or_insert(entry);
    }

    for (owner, team_paths) in teams {
        for team_path in team_paths {
            path_map
                .entry(team_path.path.clone())
                .and_modify(|existing| {
                    if !existing.owners.contains(&owner) {
                        existing.owners.push(owner.clone());
                    }

                    if existing.group.is_none() && team_path.group.is_some() {
                        existing.group = team_path.group.clone();
                    }
                })
                .or_insert_with(|| CodeOwner {
                    path: team_path.path,
                    owners: vec![owner.clone()],
                    comment: None,
                    group: team_path.group,
                });
        }
    }

    path_map
        .into_values()
        .map(|mut entry| {
            entry.owners.sort();
            entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(s: &str) -> TeamPath {
        TeamPath {
            path: s.to_string(),
            group: None,
        }
    }

    fn path_with_group(s: &str, g: &str) -> TeamPath {
        TeamPath {
            path: s.to_string(),
            group: Some(g.to_string()),
        }
    }

    #[test]
    fn test_merge_teams_creates_entries() {
        let mut teams = HashMap::new();

        teams.insert(
            Owner::Username(String::from("@petergrace")),
            vec![path("src/"), path("lib/")],
        );

        let result = merge_teams_into_entries(vec![], teams);

        assert_eq!(result.len(), 2);

        let paths: Vec<&str> = result.iter().map(|e| e.path.as_str()).collect();

        assert!(paths.contains(&"src/"));
        assert!(paths.contains(&"lib/"));
    }

    #[test]
    fn test_merge_teams_combines_owners_for_same_path() {
        let mut teams = HashMap::new();

        teams.insert(
            Owner::Username(String::from("@alice")),
            vec![path("src/")],
        );

        teams.insert(
            Owner::Team(String::from("@org/team")),
            vec![path("src/")],
        );

        let result = merge_teams_into_entries(vec![], teams);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/");
        assert_eq!(result[0].owners.len(), 2);
    }

    #[test]
    fn test_merge_preserves_entry_metadata() {
        let entries = vec![CodeOwner {
            path: String::from("src/"),
            owners: vec![Owner::Username(String::from("@admin"))],
            comment: Some(String::from("Core source")),
            group: Some(String::from("main")),
        }];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::Team(String::from("@org/team")),
            vec![path("src/")],
        );

        let result = merge_teams_into_entries(entries, teams);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].comment, Some(String::from("Core source")));
        assert_eq!(result[0].group, Some(String::from("main")));
        assert_eq!(result[0].owners.len(), 2);
    }

    #[test]
    fn test_merge_deduplicates_owners() {
        let entries = vec![CodeOwner {
            path: String::from("src/"),
            owners: vec![Owner::Username(String::from("@alice"))],
            comment: None,
            group: None,
        }];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::Username(String::from("@alice")),
            vec![path("src/")],
        );

        let result = merge_teams_into_entries(entries, teams);

        assert_eq!(result.len(), 1);

        assert_eq!(result[0].owners.len(), 1);
    }

    #[test]
    fn test_merge_teams_with_group() {
        let mut teams = HashMap::new();

        teams.insert(
            Owner::Username(String::from("@petergrace")),
            vec![path_with_group("src/", "core")],
        );

        let result = merge_teams_into_entries(vec![], teams);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "src/");
        assert_eq!(result[0].group, Some(String::from("core")));
    }

    #[test]
    fn test_entry_group_takes_precedence_over_team_group() {
        let entries = vec![CodeOwner {
            path: String::from("src/"),
            owners: vec![Owner::Username(String::from("@admin"))],
            comment: None,
            group: Some(String::from("main")),
        }];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::Team(String::from("@org/team")),
            vec![path_with_group("src/", "core")],
        );

        let result = merge_teams_into_entries(entries, teams);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].group, Some(String::from("main")));
        assert_eq!(result[0].owners.len(), 2);
    }

    #[test]
    fn test_team_group_applied_when_entry_has_none() {
        let entries = vec![CodeOwner {
            path: String::from("src/"),
            owners: vec![Owner::Username(String::from("@admin"))],
            comment: None,
            group: None,
        }];

        let mut teams = HashMap::new();

        teams.insert(
            Owner::Team(String::from("@org/team")),
            vec![path_with_group("src/", "core")],
        );

        let result = merge_teams_into_entries(entries, teams);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].group, Some(String::from("core")));
        assert_eq!(result[0].owners.len(), 2);
    }

    #[test]
    fn test_mixed_format_with_overlapping_paths() {
        use crate::structs::CodeOwners;

        const INPUT_DATA: &str = "
---
teams:
  \"@myorg/team\":
    - \"src/\"
entries:
  - path: \"src/\"
    comment: \"Core source code\"
    group: \"core\"
    owners:
      - \"@admin\"
";
        let code_owners: CodeOwners = serde_yaml::from_str(INPUT_DATA).unwrap();

        let merged = merge_teams_into_entries(code_owners.entries, code_owners.teams);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].path, "src/");
        assert_eq!(merged[0].comment, Some(String::from("Core source code")));
        assert_eq!(merged[0].group, Some(String::from("core")));
        assert_eq!(merged[0].owners.len(), 2);

        let owner_strings: Vec<String> = merged[0].owners.iter().map(|o| o.to_string()).collect();

        assert!(owner_strings.contains(&String::from("@admin")));
        assert!(owner_strings.contains(&String::from("@myorg/team")));
    }

    #[test]
    fn test_teams_with_group_yaml_parsing() {
        use crate::structs::CodeOwners;

        const INPUT_DATA: &str = "
---
teams:
  \"@petergrace\":
    - path: \"src/\"
      group: \"core\"
    - \"lib/\"
";
        let code_owners: CodeOwners = serde_yaml::from_str(INPUT_DATA).unwrap();

        let merged = merge_teams_into_entries(code_owners.entries, code_owners.teams);

        assert_eq!(merged.len(), 2);

        let src_entry = merged.iter().find(|e| e.path == "src/").unwrap();

        assert_eq!(src_entry.group, Some(String::from("core")));

        let lib_entry = merged.iter().find(|e| e.path == "lib/").unwrap();

        assert_eq!(lib_entry.group, None);
    }

    #[test]
    fn test_merge_teams_sorts_owners_alphabetically_by_type() {
        let alice = Owner::Username(String::from("@alice"));
        let cornelius = Owner::Username(String::from("@cornelius"));
        let zelda = Owner::Username(String::from("@zelda"));
        let actual_humans = Owner::Team(String::from("@actual-humans"));
        let zebras = Owner::Team(String::from("@zebras"));

        let mut teams = HashMap::new();
        teams.insert(zelda.clone(), vec![path("src/")]);
        teams.insert(cornelius.clone(), vec![path("src/")]);
        teams.insert(zebras.clone(), vec![path("src/")]);
        teams.insert(actual_humans.clone(), vec![path("src/")]);
        teams.insert(alice.clone(), vec![path("src/")]);

        let result = merge_teams_into_entries(vec![], teams);
        assert_eq!(
            result[0].owners,
            vec![alice, cornelius, zelda, actual_humans, zebras]
        );
    }
}
