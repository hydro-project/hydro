use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhPr {
    pub number: u64,
    pub head_ref_name: String,
    pub base_ref_name: String,
    pub state: PrState,
    pub is_draft: bool,
    pub url: String,
}

pub fn load_prs() -> Result<Vec<GhPr>> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--json",
            "number,headRefName,baseRefName,state,isDraft,url",
            "--limit",
            "200",
            "--state",
            "all",
        ])
        .output()
        .context("Failed to run `gh pr list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr list failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("gh output not UTF-8")?;
    let prs: Vec<GhPr> = serde_json::from_str(&stdout).context("Failed to parse gh pr list")?;
    Ok(prs)
}

pub fn create_pr(
    head: &str,
    base: &str,
    title: &str,
    body: &str,
    draft: bool,
) -> Result<u64> {
    let mut args = vec![
        "pr".to_owned(),
        "create".to_owned(),
        "--head".to_owned(),
        head.to_owned(),
        "--base".to_owned(),
        base.to_owned(),
        "--title".to_owned(),
        title.to_owned(),
        "--body".to_owned(),
        body.to_owned(),
    ];
    if draft {
        args.push("--draft".to_owned());
    }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = Command::new("gh")
        .args(&str_args)
        .output()
        .context("Failed to run `gh pr create`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr create failed: {stderr}");
    }

    // gh pr create prints the URL, extract PR number from it.
    let url = String::from_utf8_lossy(&output.stdout);
    let url = url.trim();
    // URL format: https://github.com/owner/repo/pull/123
    let pr_number = url
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .with_context(|| format!("Could not parse PR number from gh output: {url}"))?;

    Ok(pr_number)
}

pub fn edit_base(pr_number: u64, base: &str) -> Result<()> {
    let num = pr_number.to_string();
    let output = Command::new("gh")
        .args(["pr", "edit", &num, "--base", base])
        .output()
        .context("Failed to run `gh pr edit`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr edit {pr_number} --base {base} failed: {stderr}");
    }
    Ok(())
}

pub fn set_ready(pr_number: u64, ready: bool) -> Result<()> {
    let num = pr_number.to_string();
    let args = if ready {
        vec!["pr", "ready", &num]
    } else {
        vec!["pr", "ready", &num, "--undo"]
    };
    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to run `gh pr ready`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr ready failed: {stderr}");
    }
    Ok(())
}
