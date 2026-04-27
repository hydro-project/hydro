use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use anyhow::{Context, Result, bail};
use renderdag::{Ancestor, GraphRowRenderer, Renderer};

use crate::cli::TrackArgs;
use crate::gh::{self, GhPr};
use crate::jj::{self, JjState};

/// A PR node in the DAG.
#[derive(Debug)]
pub struct PrNode {
    /// GitHub PR number.
    pub number: u64,
    /// Bookmark name (head ref).
    pub bookmark: String,
    /// GitHub base ref name.
    pub base_ref: String,
    /// GitHub PR URL.
    pub url: String,
    /// GitHub PR title.
    pub title: String,
    /// Whether the PR is a draft.
    pub is_draft: bool,
    /// GitHub state.
    pub state: gh::PrState,
    /// Commit IDs that belong to this PR (tip first).
    pub commit_ids: Vec<String>,
    /// Parent PR numbers (or empty if parent is trunk).
    pub parent_prs: Vec<u64>,
    /// True if at least one parent is trunk (not another PR).
    pub has_trunk_parent: bool,
}

/// The full PR DAG.
#[derive(Debug)]
pub struct PrDag {
    /// PR number → node.
    pub nodes: BTreeMap<u64, PrNode>,
    /// Bookmark name → PR number.
    pub by_bookmark: HashMap<String, u64>,
}

/// Build the PR DAG from jj state and GitHub PRs.
pub fn build(jj_state: &JjState, gh_prs: &[GhPr]) -> Result<PrDag> {
    // Index: bookmark name → GhPr.
    let gh_by_head: HashMap<&str, &GhPr> = gh_prs
        .iter()
        .filter(|pr| pr.state == gh::PrState::Open)
        .map(|pr| (pr.head_ref_name.as_str(), pr))
        .collect();

    // Find bookmarks that are PR heads.
    // Walk jj entries, find commits with local bookmarks that match a GH PR head.
    let mut bookmark_to_commit: HashMap<String, usize> = HashMap::new();
    for (idx, entry) in jj_state.entries.iter().enumerate() {
        for bm in &entry.local_bookmarks {
            if gh_by_head.contains_key(bm.name.as_str()) {
                bookmark_to_commit.insert(bm.name.clone(), idx);
            }
        }
    }

    // For each PR bookmark, walk ancestors to find all commits in the PR.
    // A commit belongs to a PR if it has the matching `PR: #N` trailer.
    // We also find parent PRs: the first ancestor commits NOT in this PR.
    let mut nodes = BTreeMap::new();
    let mut by_bookmark = HashMap::new();

    // Index: commit_id → PR number (for commits with PR trailers).
    let mut commit_pr: HashMap<&str, u64> = HashMap::new();
    for entry in &jj_state.entries {
        if let Some(n) = jj::parse_pr_trailer(&entry.commit.description) {
            commit_pr.insert(&entry.commit.commit_id, n);
        }
    }

    for (bookmark, &tip_idx) in &bookmark_to_commit {
        let gh_pr = gh_by_head[bookmark.as_str()];
        let pr_number = gh_pr.number;

        // Walk ancestors from the tip, collecting commits that belong to this PR.
        let mut pr_commits: Vec<String> = Vec::new();
        let mut parent_prs: HashSet<u64> = HashSet::new();
        let mut has_trunk_parent = false;
        let mut queue: Vec<usize> = vec![tip_idx];
        let mut visited: HashSet<usize> = HashSet::new();

        while let Some(idx) = queue.pop() {
            if !visited.insert(idx) {
                continue;
            }
            let entry = &jj_state.entries[idx];
            let commit_belongs = commit_pr
                .get(entry.commit.commit_id.as_str())
                .is_some_and(|&n| n == pr_number);

            if commit_belongs {
                pr_commits.push(entry.commit.commit_id.clone());
                // Continue walking parents.
                for parent_id in &entry.commit.parents {
                    if let Some(&parent_idx) = jj_state.by_commit.get(parent_id) {
                        queue.push(parent_idx);
                    }
                    // If parent not in our state, it's beyond our revset (trunk).
                }
            } else if entry.immutable {
                has_trunk_parent = true;
            } else if let Some(&parent_pr) = commit_pr.get(entry.commit.commit_id.as_str()) {
                if parent_pr != pr_number {
                    parent_prs.insert(parent_pr);
                }
            } else {
                // Commit without PR trailer that isn't trunk — could be an error,
                // but for now treat as trunk boundary.
                has_trunk_parent = true;
            }
        }

        // Also check: if tip itself has no PR trailer, log a warning.
        let tip_entry = &jj_state.entries[tip_idx];
        if commit_pr
            .get(tip_entry.commit.commit_id.as_str()).is_none_or(|&n| n != pr_number)
        {
            eprintln!(
                "{}: bookmark {} tip commit {} has no matching PR trailer",
                crate::style::warn("warning"),
                crate::style::bookmark(bookmark),
                crate::style::change_id(&tip_entry.commit.commit_id),
            );
        }

        if pr_commits.is_empty() {
            eprintln!(
                "{}: {} ({}) has no commits with matching PR trailer, skipping",
                crate::style::warn("warning"),
                crate::style::pr_num(pr_number, None),
                crate::style::bookmark(bookmark),
            );
            continue;
        }

        by_bookmark.insert(bookmark.clone(), pr_number);
        nodes.insert(
            pr_number,
            PrNode {
                number: pr_number,
                bookmark: bookmark.clone(),
                base_ref: gh_pr.base_ref_name.clone(),
                url: gh_pr.url.clone(),
                title: gh_pr.title.clone(),
                is_draft: gh_pr.is_draft,
                state: gh_pr.state,
                commit_ids: pr_commits,
                parent_prs: parent_prs.into_iter().collect(),
                has_trunk_parent,
            },
        );
    }

    Ok(PrDag { nodes, by_bookmark })
}

/// Render the PR DAG as a graph to stdout.
pub fn render_log(dag: &PrDag) -> Result<()> {
    if dag.nodes.is_empty() {
        eprintln!("{}", crate::style::warn("No PRs found."));
        return Ok(());
    }

    let sorted = topo_sort_prs(dag);

    let mut renderer = GraphRowRenderer::new()
        .output()
        .with_min_row_height(1)
        .build_box_drawing();

    // Sentinel ID for trunk node.
    let trunk_id: u64 = 0;

    for &pr_num in &sorted {
        let node = &dag.nodes[&pr_num];

        let mut parents: Vec<Ancestor<u64>> = node
            .parent_prs
            .iter()
            .filter(|p| dag.nodes.contains_key(p))
            .map(|&p| Ancestor::Parent(p))
            .collect();
        if node.has_trunk_parent || parents.is_empty() {
            parents.push(Ancestor::Parent(trunk_id));
        }

        let label = format!(
            "{}  {}  {}\n{}",
            crate::style::pr_num(pr_num, Some(&node.url)),
            crate::style::status(node.is_draft),
            crate::style::bookmark(&node.bookmark),
            node.title,
        );

        let row = renderer.next_row(pr_num, parents, String::from("○"), label);
        print!("{row}");
    }

    // Render trunk node.
    let row = renderer.next_row(
        trunk_id,
        Vec::new(),
        String::from("◆"),
        crate::style::bold("trunk"),
    );
    print!("{row}");

    Ok(())
}

/// Topological sort of PR nodes (children before parents).
fn topo_sort_prs(dag: &PrDag) -> Vec<u64> {
    // "in_degree" here counts how many children point to a node as a parent.
    // We want children first, so nodes with in_degree 0 (no children depending on them
    // that haven't been emitted yet) come first — but actually we want the reverse:
    // nodes that ARE NOT parents of anything unprocessed come first.
    // This is a standard Kahn's algorithm where edges go child→parent.
    let mut in_degree: BTreeMap<u64, usize> = BTreeMap::new();

    for &pr_num in dag.nodes.keys() {
        in_degree.entry(pr_num).or_insert(0);
    }
    // Edge: child → parent. in_degree counts incoming edges (from children).
    // We want to emit children first, so we reverse: edge parent → child for topo sort,
    // meaning in_degree counts parents.
    for (&pr_num, node) in &dag.nodes {
        let parent_count = node
            .parent_prs
            .iter()
            .filter(|p| dag.nodes.contains_key(p))
            .count();
        *in_degree.entry(pr_num).or_insert(0) += parent_count;
    }

    // child_of: parent → list of children
    let mut child_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (&pr_num, node) in &dag.nodes {
        for &parent in &node.parent_prs {
            if dag.nodes.contains_key(&parent) {
                child_of.entry(parent).or_default().push(pr_num);
            }
        }
    }

    // Start with nodes that have no parents in the DAG (roots / trunk children).
    let mut queue: Vec<u64> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(n, _)| *n)
        .collect();
    queue.sort();

    let mut result = Vec::new();
    while let Some(n) = queue.pop() {
        result.push(n);
        // "removing" this node means its children lose one parent dependency.
        if let Some(children) = child_of.get(&n) {
            for &child in children {
                let d = in_degree.get_mut(&child).unwrap();
                *d -= 1;
                if *d == 0 {
                    queue.push(child);
                    queue.sort();
                }
            }
        }
    }

    // We emitted roots first, but renderdag wants children first. Reverse.
    result.reverse();
    result
}

/// A sync action to be executed.
#[derive(Debug)]
pub enum SyncAction {
    PushBookmark(String),
    UpdateBase { pr_number: u64, new_base: String },
}

impl fmt::Display for SyncAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncAction::PushBookmark(name) => write!(f, "push bookmark: {name}"),
            SyncAction::UpdateBase {
                pr_number,
                new_base,
            } => write!(f, "update PR #{pr_number} base → {new_base}"),
        }
    }
}

/// Plan sync actions by comparing local DAG state with GitHub state.
pub fn plan_sync(dag: &PrDag, jj_state: &JjState, gh_prs: &[GhPr]) -> Result<Vec<SyncAction>> {
    let gh_by_number: HashMap<u64, &GhPr> = gh_prs.iter().map(|pr| (pr.number, pr)).collect();
    let mut actions = Vec::new();

    // Build bookmark name → local commit_id and remote commit_id from jj state.
    let mut local_targets: HashMap<&str, &str> = HashMap::new();
    let mut remote_targets: HashMap<&str, &str> = HashMap::new();
    for entry in &jj_state.entries {
        for bm in &entry.local_bookmarks {
            local_targets.insert(&bm.name, &entry.commit.commit_id);
        }
        for bm in &entry.remote_bookmarks {
            // Use the first remote bookmark we find (typically "git" for colocated repos).
            remote_targets.entry(&bm.name).or_insert(&entry.commit.commit_id);
        }
    }

    for (&pr_number, node) in &dag.nodes {
        // Only push if local and remote targets differ.
        let local = local_targets.get(node.bookmark.as_str());
        let remote = remote_targets.get(node.bookmark.as_str());
        if local != remote {
            actions.push(SyncAction::PushBookmark(node.bookmark.clone()));
        }

        // Compute expected base branch.
        let expected_base = compute_expected_base(node, dag);
        if let Some(gh_pr) = gh_by_number.get(&pr_number)
            && gh_pr.base_ref_name != expected_base {
                actions.push(SyncAction::UpdateBase {
                    pr_number,
                    new_base: expected_base,
                });
            }
    }

    Ok(actions)
}

/// Compute what the GitHub base branch should be for a PR.
fn compute_expected_base(node: &PrNode, dag: &PrDag) -> String {
    // If the PR has exactly one non-trunk parent PR, base on that bookmark.
    // If it has trunk parent (or no parents), base on main.
    // If it has multiple parent PRs, pick the first one (DAG merge — imperfect but workable).
    if node.parent_prs.len() == 1 && !node.has_trunk_parent {
        let parent_num = node.parent_prs[0];
        if let Some(parent_node) = dag.nodes.get(&parent_num) {
            return parent_node.bookmark.clone();
        }
    }
    String::from("main")
}

/// Execute planned sync actions.
pub fn execute_sync(actions: &[SyncAction]) -> Result<()> {
    for action in actions {
        match action {
            SyncAction::PushBookmark(name) => {
        eprintln!("Pushing bookmark: {}", crate::style::bookmark(name));
                jj::git_push_bookmark(name)?;
            }
            SyncAction::UpdateBase {
                pr_number,
                new_base,
            } => {
                eprintln!(
                    "Updating {} base → {}",
                    crate::style::pr_num(*pr_number, None),
                    crate::style::bookmark(new_base),
                );
                gh::edit_base(*pr_number, new_base)?;
            }
        }
    }
    Ok(())
}

/// Create a new PR or update an existing one.
pub fn track_pr(
    dag: &PrDag,
    jj_state: &JjState,
    gh_prs: &[GhPr],
    args: &TrackArgs,
) -> Result<()> {
    // When --pr is given, resolve the bookmark from the GH PR's headRefName.
    let bookmark = if let Some(pr_num) = args.pr {
        let gh_pr = gh_prs
            .iter()
            .find(|pr| pr.number == pr_num)
            .with_context(|| format!("PR #{pr_num} not found on GitHub"))?;
        gh_pr.head_ref_name.clone()
    } else if let Some(ref bm) = args.bookmark {
        bm.clone()
    } else {
        // No --pr or -b: resolve from the revision's bookmark.
        String::new() // placeholder, resolved below after commit_id
    };

    // Resolve the revision.
    let rev_str = if let Some(ref r) = args.revision {
        r.clone()
    } else if !bookmark.is_empty() {
        bookmark.clone()
    } else {
        "@".to_owned()
    };

    let rev_output = std::process::Command::new("jj")
        .args(["log", "--no-graph", "-r", &rev_str, "-T", "commit_id"])
        .output()
        .context("Failed to resolve revision")?;
    if !rev_output.status.success() {
        bail!(
            "Failed to resolve revision {}: {}",
            rev_str,
            String::from_utf8_lossy(&rev_output.stderr)
        );
    }
    let commit_id = String::from_utf8(rev_output.stdout)?.trim().to_owned();

    // If bookmark was not determined yet, look it up from the commit.
    let bookmark = if bookmark.is_empty() {
        let idx = jj_state
            .by_commit
            .get(&commit_id)
            .with_context(|| format!("Commit {commit_id} not found in jj state"))?;
        let entry = &jj_state.entries[*idx];
        if let Some(bm) = entry.local_bookmarks.first() {
            bm.name.clone()
        } else {
            bail!(
                "No bookmark on revision {} — use --bookmark to specify one",
                rev_str
            );
        }
    } else {
        bookmark
    };

    // Ensure bookmark exists and points to the revision.
    jj::bookmark_set(&bookmark, &rev_str)?;

    // Track the remote bookmark if it exists (needed before push).
    if let Err(e) = jj::bookmark_track(&bookmark, "origin") {
        eprintln!("{}: {e:#}", crate::style::warn("note: bookmark track"));
    }

    // Determine base branch.
    let base = find_base_for_commit(&commit_id, jj_state, dag);

    // Push the bookmark.
    jj::git_push_bookmark(&bookmark)?;

    // Either create a new PR or use the existing one.
    let pr_number = if let Some(n) = args.pr {
        // Update existing PR — just re-stamp trailers and push.
        eprintln!(
            "Updating {} ({} → {})",
            crate::style::pr_num(n, None),
            crate::style::bookmark(&bookmark),
            crate::style::bookmark(&base),
        );
        n
    } else {
        // Check if bookmark already has a PR.
        if let Some(&existing) = dag.by_bookmark.get(&bookmark) {
            bail!(
                "Bookmark {bookmark} already has {} — use --pr {existing} to update",
                crate::style::pr_num(existing, None),
            );
        }

        // Generate title/body.
        let title = args.title.clone().unwrap_or_else(|| {
            jj_state
                .by_commit
                .get(&commit_id)
                .map(|&idx| {
                    jj_state.entries[idx]
                        .commit
                        .description
                        .lines()
                        .next()
                        .unwrap_or("untitled")
                        .to_owned()
                })
                .unwrap_or_else(|| "untitled".to_owned())
        });
        let body = args.body.clone().unwrap_or_default();

        // Create the PR on GitHub (always as draft).
        eprintln!(
            "Creating PR: {title} ({} → {}) [{}]",
            crate::style::bookmark(&bookmark),
            crate::style::bookmark(&base),
            crate::style::status(true),
        );
        let n = gh::create_pr(&bookmark, &base, &title, &body, true)?;
        eprintln!("Created {}: {title}", crate::style::pr_num(n, None));
        n
    };

    // Stamp PR trailer on all commits in the PR.
    let commits_to_stamp = find_pr_commits(&commit_id, jj_state, pr_number);
    for cid in &commits_to_stamp {
        if let Some(&idx) = jj_state.by_commit.get(cid) {
            let entry = &jj_state.entries[idx];
            let new_desc = jj::set_pr_trailer(&entry.commit.description, pr_number);
            jj::describe_stdin(&entry.commit.change_id, &new_desc)?;
        }
    }
    eprintln!(
        "Stamped {} on {} commit(s)",
        crate::style::pr_num(pr_number, None),
        commits_to_stamp.len()
    );

    Ok(())
}

/// Find the base branch for a new PR by walking parents.
fn find_base_for_commit(commit_id: &str, jj_state: &JjState, dag: &PrDag) -> String {
    let Some(&idx) = jj_state.by_commit.get(commit_id) else {
        return String::from("main");
    };
    let entry = &jj_state.entries[idx];
    for parent_id in &entry.commit.parents {
        // Check if parent has a PR trailer pointing to a known PR.
        if let Some(&parent_idx) = jj_state.by_commit.get(parent_id) {
            let parent_entry = &jj_state.entries[parent_idx];
            if let Some(pr_num) = jj::parse_pr_trailer(&parent_entry.commit.description)
                && let Some(node) = dag.nodes.get(&pr_num) {
                    return node.bookmark.clone();
                }
        }
    }
    String::from("main")
}

/// Find all commits that should be stamped with a PR trailer.
/// Walk ancestors from commit_id until we hit trunk or another PR.
fn find_pr_commits(commit_id: &str, jj_state: &JjState, _pr_number: u64) -> Vec<String> {
    let mut result = Vec::new();
    let mut queue = vec![commit_id.to_owned()];
    let mut visited = HashSet::new();

    while let Some(cid) = queue.pop() {
        if !visited.insert(cid.clone()) {
            continue;
        }
        let Some(&idx) = jj_state.by_commit.get(&cid) else {
            continue;
        };
        let entry = &jj_state.entries[idx];

        // Stop at trunk.
        if entry.immutable {
            continue;
        }
        // Stop at commits already belonging to a different PR.
        if let Some(_existing) = jj::parse_pr_trailer(&entry.commit.description) {
            continue;
        }

        result.push(cid.clone());
        for parent_id in &entry.commit.parents {
            queue.push(parent_id.clone());
        }
    }

    result
}

/// Compute the import plan: a map from change_id → pr_number.
///
/// PRs are processed in topological order (parents before children) using
/// GitHub's baseRefName to determine the DAG. Each PR walks from its
/// bookmark tip toward trunk, claiming unstamped commits and stopping
/// at any commit already assigned to another PR.
pub fn plan_import(jj_state: &JjState, gh_prs: &[GhPr]) -> BTreeMap<String, u64> {
    let open_prs: Vec<&GhPr> = gh_prs.iter().filter(|pr| pr.state == gh::PrState::Open).collect();

    // Topological sort: parents before children.
    // A PR whose base is another PR's head is a child and must come after.
    let pr_heads: HashSet<&str> = open_prs.iter().map(|pr| pr.head_ref_name.as_str()).collect();
    let mut sorted: Vec<&GhPr> = Vec::new();
    let mut remaining: Vec<&GhPr> = open_prs;
    let mut processed: HashSet<&str> = HashSet::new();

    loop {
        let before = remaining.len();
        let mut next = Vec::new();
        for pr in remaining {
            if !pr_heads.contains(pr.base_ref_name.as_str())
                || processed.contains(pr.base_ref_name.as_str())
            {
                processed.insert(&pr.head_ref_name);
                sorted.push(pr);
            } else {
                next.push(pr);
            }
        }
        remaining = next;
        if remaining.is_empty() || remaining.len() == before {
            // Append any remaining (cycles or missing base) at the end.
            sorted.extend(remaining);
            break;
        }
    }

    // Build bookmark name → jj entry index.
    let mut bookmark_to_idx: HashMap<&str, usize> = HashMap::new();
    for (idx, entry) in jj_state.entries.iter().enumerate() {
        for bm in &entry.local_bookmarks {
            bookmark_to_idx.insert(&bm.name, idx);
        }
    }

    // Process each PR: claim unstamped commits, stop at any stamped commit.
    let mut plan: BTreeMap<String, u64> = BTreeMap::new();

    for pr in &sorted {
        let Some(&tip_idx) = bookmark_to_idx.get(pr.head_ref_name.as_str()) else {
            eprintln!(
                "{}: {} ({}) — no local bookmark",
                crate::style::warn("skip"),
                crate::style::pr_num(pr.number, Some(&pr.url)),
                crate::style::bookmark(&pr.head_ref_name),
            );
            continue;
        };

        let mut queue: Vec<usize> = vec![tip_idx];
        let mut visited: HashSet<usize> = HashSet::new();

        while let Some(idx) = queue.pop() {
            if !visited.insert(idx) {
                continue;
            }
            let entry = &jj_state.entries[idx];

            if entry.immutable {
                continue;
            }

            // Stop at commits already assigned to another PR.
            if let Some(&existing) = plan.get(&entry.commit.change_id) {
                if existing != pr.number {
                    continue;
                }
                // Already ours — keep walking.
            } else {
                // Unstamped — claim it.
                plan.insert(entry.commit.change_id.clone(), pr.number);
            }

            for parent_id in &entry.commit.parents {
                if let Some(&pidx) = jj_state.by_commit.get(parent_id) {
                    queue.push(pidx);
                }
            }
        }
    }

    // Filter out changes that already have the correct trailer.
    plan.into_iter()
        .filter(|(change_id, pr_number)| {
            let Some(&idx) = jj_state.by_change.get(change_id) else {
                return true;
            };
            let existing = jj::parse_pr_trailer(&jj_state.entries[idx].commit.description);
            existing != Some(*pr_number)
        })
        .collect()
}

/// Import existing GitHub PRs by stamping PR trailers on local commits.
pub fn import_prs(jj_state: &JjState, gh_prs: &[GhPr], dry_run: bool) -> Result<()> {
    let plan = plan_import(jj_state, gh_prs);

    if plan.is_empty() {
        eprintln!("Nothing to import — all PRs already have correct trailers.");
        return Ok(());
    }

    // Build PR number → URL lookup for display.
    let pr_urls: HashMap<u64, &str> = gh_prs
        .iter()
        .map(|pr| (pr.number, pr.url.as_str()))
        .collect();

    // Phase 2: Display plan grouped by PR.
    let mut by_pr: BTreeMap<u64, Vec<&str>> = BTreeMap::new();
    for (change_id, pr_number) in &plan {
        by_pr.entry(*pr_number).or_default().push(change_id);
    }
    // Sort each PR's changes by their position in jj log (topological order).
    for changes in by_pr.values_mut() {
        changes.sort_by_key(|cid| *jj_state.by_change.get(*cid).expect("bug: change_id not in jj state"));
    }

    eprintln!("{}", crate::style::bold(&format!("{} commit(s) to update:", plan.len())));
    for (pr_number, change_ids) in &by_pr {
        let url = pr_urls.get(pr_number).copied();
        eprintln!("  {} ({} commit(s))", crate::style::pr_num(*pr_number, url), change_ids.len());
        for change_id in change_ids {
            let idx = *jj_state.by_change.get(*change_id).expect("bug: change_id not in jj state");
            let first_line = jj_state.entries[idx]
                .commit
                .description
                .lines()
                .next()
                .unwrap_or("(empty)");
            eprintln!(
                "    {} {first_line}",
                crate::style::change_id(change_id),
            );
        }
    }

    // Phase 3: Apply.
    if dry_run {
        eprintln!(
            "\n{}",
            crate::style::warn(&format!("Dry run: would stamp {} commit(s)", plan.len()))
        );
    } else {
        for (change_id, pr_number) in &plan {
            let idx = jj_state.by_change[change_id];
            let entry = &jj_state.entries[idx];
            let new_desc = jj::set_pr_trailer(&entry.commit.description, *pr_number);
            jj::describe_stdin(change_id, &new_desc)?;
        }
        eprintln!("\nStamped {} commit(s)", plan.len());
    }

    Ok(())
}
