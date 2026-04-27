#[cfg(test)]
mod tests {
    use crate::gh::{GhPr, PrState};
    use crate::jj::{JjBookmark, JjCommit, JjLogEntry, JjRemoteBookmark, JjState};
    use crate::pr_dag;

    /// Helper to build a JjLogEntry.
    fn entry(
        commit_id: &str,
        change_id: &str,
        parents: &[&str],
        description: &str,
        local_bookmarks: &[&str],
        immutable: bool,
    ) -> JjLogEntry {
        JjLogEntry {
            commit: JjCommit {
                commit_id: commit_id.to_owned(),
                change_id: change_id.to_owned(),
                parents: parents.iter().map(|s| (*s).to_owned()).collect(),
                description: description.to_owned(),
            },
            local_bookmarks: local_bookmarks
                .iter()
                .map(|name| JjBookmark {
                    name: (*name).to_owned(),
                    target: vec![commit_id.to_owned()],
                })
                .collect(),
            remote_bookmarks: vec![],
            immutable,
        }
    }

    /// Helper to add remote bookmarks to an entry.
    fn with_remote(mut entry: JjLogEntry, name: &str, remote: &str) -> JjLogEntry {
        entry.remote_bookmarks.push(JjRemoteBookmark {
            name: name.to_owned(),
            remote: Some(remote.to_owned()),
            target: vec![entry.commit.commit_id.clone()],
        });
        entry
    }

    fn gh_pr(number: u64, head: &str, base: &str) -> GhPr {
        GhPr {
            number,
            head_ref_name: head.to_owned(),
            base_ref_name: base.to_owned(),
            state: PrState::Open,
            is_draft: false,
            url: format!("https://github.com/test/repo/pull/{number}"),
            title: format!("PR #{number}"),
        }
    }

    // ---- DAG building tests ----

    #[test]
    fn build_single_pr_on_trunk() {
        // trunk <- c1 (PR #1) <- c2 (PR #1, bookmark: feat)
        let entries = vec![
            entry("c2", "ch2", &["c1"], "feat\n\nPR: #1\n", &["feat"], false),
            entry("c1", "ch1", &["trunk"], "first\n\nPR: #1\n", &[], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        assert_eq!(dag.nodes.len(), 1);

        let node = &dag.nodes[&1];
        assert_eq!(node.bookmark, "feat");
        assert_eq!(node.commit_ids.len(), 2);
        assert!(node.has_trunk_parent);
        assert!(node.parent_prs.is_empty());
    }

    #[test]
    fn build_stacked_prs() {
        // trunk <- a1 (PR #1, bookmark: feat-a) <- b1 (PR #2, bookmark: feat-b)
        let entries = vec![
            entry("b1", "chb1", &["a1"], "b\n\nPR: #2\n", &["feat-b"], false),
            entry("a1", "cha1", &["trunk"], "a\n\nPR: #1\n", &["feat-a"], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat-a", "main"), gh_pr(2, "feat-b", "feat-a")];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        assert_eq!(dag.nodes.len(), 2);

        let node_a = &dag.nodes[&1];
        assert!(node_a.has_trunk_parent);
        assert!(node_a.parent_prs.is_empty());

        let node_b = &dag.nodes[&2];
        assert!(!node_b.has_trunk_parent);
        assert_eq!(node_b.parent_prs, vec![1]);
    }

    #[test]
    fn build_diamond_dag() {
        // trunk <- a (PR #1) <- c (PR #3, two parents)
        //       <- b (PR #2) <-/
        let entries = vec![
            entry("c1", "chc1", &["a1", "b1"], "c\n\nPR: #3\n", &["feat-c"], false),
            entry("a1", "cha1", &["trunk"], "a\n\nPR: #1\n", &["feat-a"], false),
            entry("b1", "chb1", &["trunk"], "b\n\nPR: #2\n", &["feat-b"], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![
            gh_pr(1, "feat-a", "main"),
            gh_pr(2, "feat-b", "main"),
            gh_pr(3, "feat-c", "feat-a"),
        ];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        assert_eq!(dag.nodes.len(), 3);

        let node_c = &dag.nodes[&3];
        let mut parents = node_c.parent_prs.clone();
        parents.sort();
        assert_eq!(parents, vec![1, 2]);
    }

    // ---- Sync planning tests ----

    #[test]
    fn sync_no_changes_needed() {
        let entries = vec![
            with_remote(
                entry("c1", "ch1", &["trunk"], "feat\n\nPR: #1\n", &["feat"], false),
                "feat",
                "git",
            ),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        let actions = pr_dag::plan_sync(&dag, &jj, &prs).unwrap();
        assert!(actions.is_empty(), "expected no actions, got: {actions:?}");
    }

    #[test]
    fn sync_push_needed_when_remote_differs() {
        // Local bookmark at c2, remote at c1.
        let entries = vec![
            entry("c2", "ch2", &["c1"], "update\n\nPR: #1\n", &["feat"], false),
            with_remote(
                entry("c1", "ch1", &["trunk"], "feat\n\nPR: #1\n", &[], false),
                "feat",
                "git",
            ),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        let actions = pr_dag::plan_sync(&dag, &jj, &prs).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(&actions[0], pr_dag::SyncAction::PushBookmark(name) if name == "feat"));
    }

    #[test]
    fn sync_update_base_when_wrong() {
        // PR #2 based on feat-a, but GitHub says base is "main".
        let entries = vec![
            with_remote(
                entry("b1", "chb1", &["a1"], "b\n\nPR: #2\n", &["feat-b"], false),
                "feat-b",
                "git",
            ),
            with_remote(
                entry("a1", "cha1", &["trunk"], "a\n\nPR: #1\n", &["feat-a"], false),
                "feat-a",
                "git",
            ),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![
            gh_pr(1, "feat-a", "main"),
            gh_pr(2, "feat-b", "main"), // wrong base
        ];

        let dag = pr_dag::build(&jj, &prs).unwrap();
        let actions = pr_dag::plan_sync(&dag, &jj, &prs).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], pr_dag::SyncAction::UpdateBase { pr_number: 2, new_base } if new_base == "feat-a")
        );
    }

    // ---- Import planning tests ----

    #[test]
    fn import_stamps_unstamped_commits() {
        let entries = vec![
            entry("c2", "ch2", &["c1"], "second commit\n", &["feat"], false),
            entry("c1", "ch1", &["trunk"], "first commit\n", &[], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let plan = pr_dag::plan_import(&jj, &prs);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan["ch1"], 1);
        assert_eq!(plan["ch2"], 1);
    }

    #[test]
    fn import_skips_already_stamped() {
        let entries = vec![
            entry("c1", "ch1", &["trunk"], "feat\n\nPR: #1\n", &["feat"], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let plan = pr_dag::plan_import(&jj, &prs);
        assert!(plan.is_empty());
    }

    #[test]
    fn import_parent_reclaims_from_child() {
        // trunk <- a1 (bookmark: feat-a) <- b1 (bookmark: feat-b)
        // Both unstamped. Child processed first would stamp a1 as PR #2,
        // but parent should reclaim it as PR #1.
        let entries = vec![
            entry("b1", "chb1", &["a1"], "child\n", &["feat-b"], false),
            entry("a1", "cha1", &["trunk"], "parent\n", &["feat-a"], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat-a", "main"), gh_pr(2, "feat-b", "feat-a")];

        let plan = pr_dag::plan_import(&jj, &prs);
        // a1 should belong to PR #1 (parent reclaims), b1 to PR #2.
        assert_eq!(plan["cha1"], 1);
        assert_eq!(plan["chb1"], 2);
    }

    #[test]
    fn import_stops_at_trunk() {
        let entries = vec![
            entry("c1", "ch1", &["trunk"], "feat\n", &["feat"], false),
            entry("trunk", "chtrunk", &[], "trunk\n", &["main"], true),
        ];
        let jj = JjState::new(entries);
        let prs = vec![gh_pr(1, "feat", "main")];

        let plan = pr_dag::plan_import(&jj, &prs);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan["ch1"], 1);
        // trunk should NOT be in the plan.
        assert!(!plan.contains_key("chtrunk"));
    }
}
