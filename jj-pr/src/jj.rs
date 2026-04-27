use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// Raw commit data from `json(self)`.
#[derive(Debug, Deserialize)]
pub struct JjCommit {
    pub commit_id: String,
    pub change_id: String,
    pub parents: Vec<String>,
    pub description: String,
}

/// Bookmark reference from `json(local_bookmarks)`.
#[derive(Debug, Deserialize)]
pub struct JjBookmark {
    pub name: String,
    pub target: Vec<String>,
}

/// One line of JSONL output from our composite template.
#[derive(Debug, Deserialize)]
pub struct JjLogEntry {
    pub commit: JjCommit,
    pub local_bookmarks: Vec<JjBookmark>,
    pub immutable: bool,
}

/// Parsed PR trailer value, e.g. `PR: #1234` → `1234`.
pub fn parse_pr_trailer(description: &str) -> Option<u64> {
    // Trailers are `Key: Value` lines at the end of the description.
    // We look for `PR: #<number>` anywhere in the trailer block.
    for line in description.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            break; // End of trailer block (blank line separator).
        }
        if let Some(value) = line.strip_prefix("PR: #") {
            if let Ok(n) = value.trim().parse::<u64>() {
                return Some(n);
            }
        }
    }
    None
}

/// Update or append a `PR: #N` trailer in a description.
pub fn set_pr_trailer(description: &str, pr_number: u64) -> String {
    let trailer_line = format!("PR: #{pr_number}");
    let lines: Vec<&str> = description.lines().collect();

    // Find existing PR trailer and replace it.
    for (i, line) in lines.iter().enumerate().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.starts_with("PR: #") {
            let mut new_lines: Vec<&str> = lines[..i].to_vec();
            new_lines.push(&trailer_line);
            new_lines.extend_from_slice(&lines[i + 1..]);
            let mut result = new_lines.join("\n");
            if description.ends_with('\n') {
                result.push('\n');
            }
            return result;
        }
    }

    // No existing trailer — append after a blank line if needed.
    let trimmed = description.trim_end();
    if trimmed.is_empty() {
        return format!("{trailer_line}\n");
    }
    // Check if there's already a trailer block (last non-empty line contains `: `).
    let last_line = trimmed.lines().last().unwrap_or("");
    if last_line.contains(": ") {
        // Append to existing trailer block.
        format!("{trimmed}\n{trailer_line}\n")
    } else {
        // Start new trailer block with blank line separator.
        format!("{trimmed}\n\n{trailer_line}\n")
    }
}

/// Full state loaded from jj.
#[derive(Debug)]
pub struct JjState {
    pub entries: Vec<JjLogEntry>,
    /// commit_id → index into entries
    pub by_commit: HashMap<String, usize>,
    /// change_id → index into entries
    pub by_change: HashMap<String, usize>,
}

const JJ_TEMPLATE: &str = r#""{\"commit\": " ++ json(self) ++ ", \"local_bookmarks\": " ++ json(local_bookmarks) ++ ", \"immutable\": " ++ json(self.immutable()) ++ "}\n""#;

pub fn load_state() -> Result<JjState> {
    load_state_with_revset("trunk().. | trunk()")
}

pub fn load_state_with_revset(revset: &str) -> Result<JjState> {
    let output = Command::new("jj")
        .args(["log", "--no-graph", "-r", revset, "-T", JJ_TEMPLATE])
        .output()
        .context("Failed to run `jj log`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj log failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("jj log output not UTF-8")?;
    let mut entries = Vec::new();
    let mut by_commit = HashMap::new();
    let mut by_change = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: JjLogEntry =
            serde_json::from_str(line).with_context(|| format!("Failed to parse: {line}"))?;
        let idx = entries.len();
        by_commit.insert(entry.commit.commit_id.clone(), idx);
        by_change.insert(entry.commit.change_id.clone(), idx);
        entries.push(entry);
    }

    Ok(JjState {
        entries,
        by_commit,
        by_change,
    })
}

/// Set the description of a revision via `jj describe --stdin`.
pub fn describe_stdin(revision: &str, description: &str) -> Result<()> {
    use std::io::Write;
    let mut child = Command::new("jj")
        .args(["describe", revision, "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn `jj describe`")?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(description.as_bytes())
        .context("Failed to write to jj describe stdin")?;

    let output = child
        .wait_with_output()
        .context("Failed to wait for `jj describe`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj describe failed: {stderr}");
    }
    Ok(())
}

/// Push a bookmark to the remote.
pub fn git_push_bookmark(bookmark: &str) -> Result<()> {
    let output = Command::new("jj")
        .args(["git", "push", "--bookmark", bookmark])
        .output()
        .context("Failed to run `jj git push`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj git push --bookmark {bookmark} failed: {stderr}");
    }
    Ok(())
}

/// Set a bookmark to point at a revision.
pub fn bookmark_set(name: &str, revision: &str) -> Result<()> {
    let output = Command::new("jj")
        .args(["bookmark", "set", name, "-r", revision])
        .output()
        .context("Failed to run `jj bookmark set`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj bookmark set {name} failed: {stderr}");
    }
    Ok(())
}

/// Track a remote bookmark.
pub fn bookmark_track(name: &str, remote: &str) -> Result<()> {
    let refname = format!("{name}@{remote}");
    let output = Command::new("jj")
        .args(["bookmark", "track", &refname])
        .output()
        .context("Failed to run `jj bookmark track`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("jj bookmark track {refname} failed: {stderr}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pr_trailer_basic() {
        assert_eq!(parse_pr_trailer("some commit\n\nPR: #42\n"), Some(42));
    }

    #[test]
    fn parse_pr_trailer_with_other_trailers() {
        let desc = "fix bug\n\nCo-authored-by: Alice\nPR: #123\n";
        assert_eq!(parse_pr_trailer(desc), Some(123));
    }

    #[test]
    fn parse_pr_trailer_missing() {
        assert_eq!(parse_pr_trailer("just a commit message\n"), None);
    }

    #[test]
    fn parse_pr_trailer_no_trailing_newline() {
        assert_eq!(parse_pr_trailer("msg\n\nPR: #7"), Some(7));
    }

    #[test]
    fn set_pr_trailer_append_new() {
        let result = set_pr_trailer("add feature\n", 99);
        assert_eq!(result, "add feature\n\nPR: #99\n");
    }

    #[test]
    fn set_pr_trailer_replace_existing() {
        let result = set_pr_trailer("fix\n\nPR: #10\n", 20);
        assert_eq!(result, "fix\n\nPR: #20\n");
    }

    #[test]
    fn set_pr_trailer_append_to_existing_trailers() {
        let result = set_pr_trailer("fix\n\nCo-authored-by: Bob\n", 55);
        assert_eq!(result, "fix\n\nCo-authored-by: Bob\nPR: #55\n");
    }

    #[test]
    fn set_pr_trailer_empty_description() {
        let result = set_pr_trailer("", 1);
        assert_eq!(result, "PR: #1\n");
    }
}
