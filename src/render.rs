use crate::structs::{CodeOwner, UNGROUPED};

/// Renders the body of the CODEOWNERS file (everything after the version
/// header). Entries MUST already be sorted into final render order. Blocks are
/// driven entirely by the entries present: a header opens when the block label
/// changes, and every opened block is closed with a matching footer.
pub(crate) fn render_body(entries: &[CodeOwner]) -> String {
    let longest_path = entries
        .iter()
        .map(|c| c.path.as_os_str().len() + if c.negate { 1 } else { 0 })
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let mut current: Option<String> = None;

    for co in entries {
        let label = block_label(co);
        if current.as_deref() != Some(label.as_str()) {
            if let Some(prev) = current.take() {
                out.push_str(&format!("### END {}\n", prev));
            }
            out.push_str(&format!("####### BEGIN {}\n", label));
            current = Some(label);
        }

        if let Some(comment) = &co.comment {
            out.push_str(&format!("# {}\n", comment));
        }

        let display_path = if co.negate {
            format!("!{}", co.path.display())
        } else {
            co.path.display().to_string()
        };

        let mut owners = String::new();
        for o in &co.owners {
            owners = format!("{} {}", owners, o);
        }

        out.push_str(&format!(
            "{:width$} {}\n",
            display_path,
            owners,
            width = longest_path
        ));
    }

    if let Some(prev) = current.take() {
        out.push_str(&format!("### END {}\n", prev));
    }

    out
}

/// The block label for an entry. The reserved `ungrouped` name (explicit or
/// implied) renders as the `UNGROUPED` block; everything else as `GROUP <NAME>`.
fn block_label(co: &CodeOwner) -> String {
    let name = co.group_name();
    if name == UNGROUPED {
        "UNGROUPED".to_string()
    } else {
        format!("GROUP {}", name.to_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{CodeOwner, Owner};
    use std::path::PathBuf;

    fn co(
        path: &str,
        group: Option<&str>,
        negate: bool,
        comment: Option<&str>,
        owner: Option<&str>,
    ) -> CodeOwner {
        CodeOwner {
            path: PathBuf::from(path),
            negate,
            owners: owner
                .map(|o| vec![Owner::Team(o.to_string())])
                .unwrap_or_default(),
            comment: comment.map(|c| c.to_string()),
            group: group.map(|g| g.to_string()),
        }
    }

    // Returns just the block header/footer lines, in order.
    fn structure(body: &str) -> Vec<&str> {
        body.lines()
            .filter(|l| l.starts_with("#######") || l.starts_with("### END"))
            .collect()
    }

    #[test]
    fn test_render_emits_contiguous_blocks_with_matching_footers() {
        // already in final sorted order: ungrouped, then core, then docs
        let entries = vec![
            co("*", None, false, None, Some("@org/default")),
            co(
                "src/",
                Some("core"),
                false,
                Some("core code"),
                Some("@org/core"),
            ),
            co("lib/", Some("core"), false, None, Some("@org/core")),
            co("build/", Some("docs"), true, None, None),
        ];
        let body = render_body(&entries);

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
        // comment renders above its entry
        assert!(body.contains("# core code\n"));
        // negate prefix renders
        assert!(body.lines().any(|l| l.starts_with("!build/")));
        // owners render on the catch-all line
        assert!(
            body.lines()
                .any(|l| l.starts_with("*") && l.contains("@org/default"))
        );
    }

    #[test]
    fn test_explicit_ungrouped_group_uses_ungrouped_block() {
        let entries = vec![co("x", Some("ungrouped"), false, None, Some("@org/x"))];
        let body = render_body(&entries);
        assert_eq!(
            structure(&body),
            vec!["####### BEGIN UNGROUPED", "### END UNGROUPED"]
        );
    }

    #[test]
    fn test_multiple_owners_render_space_separated() {
        let entries = vec![CodeOwner {
            path: PathBuf::from("*"),
            negate: false,
            owners: vec![
                Owner::Team("@org/a".to_string()),
                Owner::Username("@bob".to_string()),
            ],
            comment: None,
            group: None,
        }];
        let body = render_body(&entries);
        // both owners present on the entry line, space-separated in order
        assert!(
            body.lines()
                .any(|l| l.starts_with("*") && l.contains("@org/a @bob")),
            "expected '@org/a @bob' on the entry line, got:\n{}",
            body
        );
    }
}
