use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use anyhow::{Context, Result, bail};
use renderdag::{GraphRenderer, Node, RenderConfig};

use crate::cli::CreateArgs;
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
    /// Whether the PR is a draft.
    pub is_draft: bool,
    /// GitHub state (OPEN, MERGED, CLOSED).
    pub state: String,
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
        .filter(|pr| pr.state == "OPEN")
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
        if !commit_pr
            .get(tip_entry.commit.commit_id.as_str())
            .is_some_and(|&n| n == pr_number)
        {
            eprintln!(
                "warning: bookmark {bookmark} (PR #{pr_number}) tip commit {} has no matching PR trailer",
                &tip_entry.commit.commit_id[..12]
            );
        }

        if pr_commits.is_empty() {
            eprintln!(
                "warning: PR #{pr_number} ({bookmark}) has no commits with matching PR trailer, skipping"
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
                is_draft: gh_pr.is_draft,
                state: gh_pr.state.clone(),
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
        eprintln!("No PRs found.");
        return Ok(());
    }

    // Build renderdag Node list. Each PR becomes a node; trunk is the root.
    // renderdag wants nodes in topological order (children before parents).
    let sorted = topo_sort_prs(dag);

    let trunk_id = "trunk".to_string();
    let mut nodes: Vec<Node> = Vec::new();

    for &pr_num in &sorted {
        let node = &dag.nodes[&pr_num];
        let id = pr_num.to_string();
        let mut parents: Vec<String> = node
            .parent_prs
            .iter()
            .filter(|p| dag.nodes.contains_key(p))
            .map(|p| p.to_string())
            .collect();
        if node.has_trunk_parent || parents.is_empty() {
            parents.push(trunk_id.clone());
        }
        nodes.push(Node::new(id, parents));
    }

    // Add trunk as the root node (no parents).
    nodes.push(Node::new(trunk_id, Vec::new()));

    // Render.
    let config = RenderConfig::default();
    let mut renderer = GraphRenderer::new(config);
    let output = renderer.render_to_string(&nodes);

    // The default rendering uses node IDs as labels. We want richer labels.
    // renderdag doesn't support custom labels directly in render_to_string,
    // so we post-process: replace each node ID with our label.
    let mut label_map: HashMap<String, String> = HashMap::new();
    for (&pr_num, node) in &dag.nodes {
        let status = if node.is_draft { "draft" } else { "ready" };
        label_map.insert(
            pr_num.to_string(),
            format!("{}  PR #{}  ({})", node.bookmark, pr_num, status),
        );
    }
    label_map.insert("trunk".to_string(), "trunk".to_string());

    // Replace IDs in output. renderdag puts the ID after the glyph on each row.
    let mut result = output;
    // Replace longest IDs first to avoid partial matches.
    let mut ids: Vec<&String> = label_map.keys().collect();
    ids.sort_by(|a, b| b.len().cmp(&a.len()));
    for id in ids {
        let label = &label_map[id];
        result = result.replace(id, label);
    }

    print!("{result}");
    Ok(())
}

/// Topological sort of PR nodes (children before parents).
fn topo_sort_prs(dag: &PrDag) -> Vec<u64> {
    // "in_degree" here counts how many children point to a node as a parent.
    // We want children first, so nodes with in_degree 0 (no children depending on them
    // that haven't been emitted yet) come first — but actually we want the reverse:
    // nodes that ARE NOT parents of anything unprocessed come first.
    // This is a standard Kahn's algorithm where edges go child→parent.
    let mut in_degree: HashMap<u64, usize> = HashMap::new();

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
    let mut child_of: HashMap<u64, Vec<u64>> = HashMap::new();
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
pub fn plan_sync(dag: &PrDag, gh_prs: &[GhPr]) -> Result<Vec<SyncAction>> {
    let gh_by_number: HashMap<u64, &GhPr> = gh_prs.iter().map(|pr| (pr.number, pr)).collect();
    let mut actions = Vec::new();

    for (&pr_number, node) in &dag.nodes {
        // Always push bookmarks (jj git push is idempotent if nothing changed).
        actions.push(SyncAction::PushBookmark(node.bookmark.clone()));

        // Compute expected base branch.
        let expected_base = compute_expected_base(node, dag);
        if let Some(gh_pr) = gh_by_number.get(&pr_number) {
            if gh_pr.base_ref_name != expected_base {
                actions.push(SyncAction::UpdateBase {
                    pr_number,
                    new_base: expected_base,
                });
            }
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
                eprintln!("Pushing bookmark: {name}");
                jj::git_push_bookmark(name)?;
            }
            SyncAction::UpdateBase {
                pr_number,
                new_base,
            } => {
                eprintln!("Updating PR #{pr_number} base → {new_base}");
                gh::edit_base(*pr_number, new_base)?;
            }
        }
    }
    Ok(())
}

/// Create a new PR.
pub fn create_pr(
    dag: &PrDag,
    jj_state: &JjState,
    _gh_prs: &[GhPr],
    args: &CreateArgs,
) -> Result<()> {
    // Resolve the revision. If -r is given, use it. Otherwise, if -b is given
    // and the bookmark exists, use the bookmark. Otherwise default to @.
    let rev_str = match (&args.revision, &args.bookmark) {
        (Some(r), _) => r.clone(),
        (None, Some(bm)) => bm.clone(), // jj will resolve bookmark name to its target
        (None, None) => "@".to_string(),
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
    let commit_id = String::from_utf8(rev_output.stdout)?.trim().to_string();

    // Determine bookmark name.
    let bookmark = if let Some(ref bm) = args.bookmark {
        bm.clone()
    } else {
        // Check if the commit already has a local bookmark.
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
    };

    // Check if bookmark already has a PR.
    if dag.by_bookmark.contains_key(&bookmark) {
        bail!("Bookmark {bookmark} already has a PR");
    }

    // Ensure bookmark exists and points to the revision.
    jj::bookmark_set(&bookmark, &rev_str)?;

    // Determine base branch.
    // Walk parents of the commit to find the nearest PR or trunk.
    let base = find_base_for_commit(&commit_id, jj_state, dag);

    // Determine draft status: draft if base is another PR (not main/trunk).
    let draft = base != "main";

    // Push the bookmark.
    jj::git_push_bookmark(&bookmark)?;

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
                    .to_string()
            })
            .unwrap_or_else(|| "untitled".to_string())
    });
    let body = args.body.clone().unwrap_or_default();

    // Create the PR on GitHub.
    let status = if draft { "draft" } else { "ready" };
    eprintln!("Creating PR: {title} ({bookmark} → {base}) [{status}]");
    let pr_number = gh::create_pr(&bookmark, &base, &title, &body, draft)?;
    eprintln!("Created PR #{pr_number}: {title}");

    // Stamp PR trailer on all commits in the PR.
    // For now, stamp the tip commit. Walk ancestors until we hit trunk or another PR.
    let commits_to_stamp = find_pr_commits(&commit_id, jj_state, pr_number);
    for cid in &commits_to_stamp {
        if let Some(&idx) = jj_state.by_commit.get(cid) {
            let entry = &jj_state.entries[idx];
            let new_desc = jj::set_pr_trailer(&entry.commit.description, pr_number);
            jj::describe_stdin(&entry.commit.change_id, &new_desc)?;
        }
    }
    eprintln!(
        "Stamped PR: #{pr_number} on {} commit(s)",
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
            if let Some(pr_num) = jj::parse_pr_trailer(&parent_entry.commit.description) {
                if let Some(node) = dag.nodes.get(&pr_num) {
                    return node.bookmark.clone();
                }
            }
        }
    }
    String::from("main")
}

/// Find all commits that should be stamped with a PR trailer.
/// Walk ancestors from commit_id until we hit trunk or another PR.
fn find_pr_commits(commit_id: &str, jj_state: &JjState, _pr_number: u64) -> Vec<String> {
    let mut result = Vec::new();
    let mut queue = vec![commit_id.to_string()];
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

/// Import existing GitHub PRs by stamping PR trailers on local commits.
///
/// For each open GH PR whose head branch matches a local bookmark,
/// walk ancestors from the bookmark tip to trunk, stamping `PR: #N`.
/// Overwrites any existing PR trailer — this means processing order
/// doesn't matter: if a child is processed before its parent, the
/// parent will reclaim its commits by overwriting the child's trailer.
pub fn import_prs(jj_state: &JjState, gh_prs: &[GhPr], dry_run: bool) -> Result<()> {
    let open_prs: Vec<&GhPr> = gh_prs.iter().filter(|pr| pr.state == "OPEN").collect();

    // Build bookmark name → jj entry index.
    let mut bookmark_to_idx: HashMap<&str, usize> = HashMap::new();
    for (idx, entry) in jj_state.entries.iter().enumerate() {
        for bm in &entry.local_bookmarks {
            bookmark_to_idx.insert(&bm.name, idx);
        }
    }

    // Process each PR: walk ancestors from tip, stamp trailers.
    let mut total_stamped = 0usize;
    for pr in &open_prs {
        let Some(&tip_idx) = bookmark_to_idx.get(pr.head_ref_name.as_str()) else {
            eprintln!(
                "skip: PR #{} ({}) — no local bookmark",
                pr.number, pr.head_ref_name
            );
            continue;
        };

        let pr_number = pr.number;
        let bookmark = &pr.head_ref_name;

        let mut to_stamp: Vec<usize> = Vec::new();
        let mut queue: Vec<usize> = vec![tip_idx];
        let mut visited: HashSet<usize> = HashSet::new();

        while let Some(idx) = queue.pop() {
            if !visited.insert(idx) {
                continue;
            }
            let entry = &jj_state.entries[idx];

            // Stop at trunk/immutable.
            if entry.immutable {
                continue;
            }

            // Check existing trailer.
            if let Some(existing) = jj::parse_pr_trailer(&entry.commit.description) {
                if existing == pr_number {
                    // Already correct — skip but keep walking parents.
                    for parent_id in &entry.commit.parents {
                        if let Some(&pidx) = jj_state.by_commit.get(parent_id) {
                            queue.push(pidx);
                        }
                    }
                    continue;
                }
                // Different PR — overwrite it (parent will reclaim later).
            }

            to_stamp.push(idx);
            for parent_id in &entry.commit.parents {
                if let Some(&pidx) = jj_state.by_commit.get(parent_id) {
                    queue.push(pidx);
                }
            }
        }

        if to_stamp.is_empty() {
            eprintln!("PR #{pr_number} ({bookmark}): already imported");
            continue;
        }

        eprintln!(
            "PR #{pr_number} ({bookmark}): stamping {} commit(s)",
            to_stamp.len()
        );

        if !dry_run {
            for &idx in &to_stamp {
                let entry = &jj_state.entries[idx];
                let new_desc = jj::set_pr_trailer(&entry.commit.description, pr_number);
                jj::describe_stdin(&entry.commit.change_id, &new_desc)?;
            }
        } else {
            for &idx in &to_stamp {
                let entry = &jj_state.entries[idx];
                let short_id = &entry.commit.change_id[..12];
                let first_line = entry
                    .commit
                    .description
                    .lines()
                    .next()
                    .unwrap_or("(empty)");
                eprintln!("  {short_id} {first_line}");
            }
        }

        total_stamped += to_stamp.len();
    }

    if dry_run {
        eprintln!("\nDry run: would stamp {total_stamped} commit(s) total");
    } else {
        eprintln!("\nStamped {total_stamped} commit(s) total");
    }

    Ok(())
}
