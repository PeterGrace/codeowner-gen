use crate::structs::{CodeOwner, CodeOwners, sort_entries};
use crate::{owner_groups, teams};
use std::collections::HashMap;
use std::path::PathBuf;

/// Resolves a parsed config into the final, render-ordered list of entries.
///
/// Steps: capture each entry's authored position (its index in the original
/// `entries:` list) BEFORE merging destroys order, expand `owner_groups`
/// aliases, merge the `teams:` shorthand, then sort under the author-owned
/// ordering model.
pub(crate) fn process(code_owners: CodeOwners) -> Result<Vec<CodeOwner>, String> {
    let mut author_order: HashMap<PathBuf, usize> = HashMap::new();
    for (i, entry) in code_owners.entries.iter().enumerate() {
        author_order.entry(entry.path.clone()).or_insert(i);
    }

    let (expanded_entries, expanded_teams) = owner_groups::expand_owner_groups(
        &code_owners.owner_groups,
        code_owners.entries,
        code_owners.teams,
    )?;

    let mut entries = teams::merge_teams_into_entries(expanded_entries, expanded_teams);
    sort_entries(&mut entries, &author_order);
    Ok(entries)
}
