/**
 * Shared utilities for managing benchmark results comments in GitHub PRs.
 * 
 * This module provides functions to find, extract, and update benchmark
 * results in PR comments, maintaining a history of all benchmark runs.
 */

/**
 * Finds the existing benchmark results comment in the PR.
 * @param {import('@octokit/rest').Octokit} github - GitHub API client
 * @param {Object} context - GitHub Actions context
 * @returns {Promise<Object|null>} The comment object if found, null otherwise
 */
async function findBenchmarkComment(github, context) {
  const { data: comments } = await github.rest.issues.listComments({
    owner: context.repo.owner,
    repo: context.repo.repo,
    issue_number: context.issue.number,
  });
  return comments.find(comment =>
    comment.user.type === 'Bot' && comment.body.includes('📊 Benchmark Results')
  ) || null;
}

/**
 * Extracts the run history section from an existing comment body.
 * @param {string} commentBody - The full comment body text
 * @returns {string} The extracted history entries (lines between "### Run History:" and timestamp)
 */
function extractRunHistory(commentBody) {
  const historyMatch = commentBody.match(/### Run History:\n([\s\S]*?)\n\n<sub>/);
  return historyMatch ? historyMatch[1] : '';
}

/**
 * Creates the formatted comment body with status and run history.
 * @param {string} status - Status message (e.g., "⏳ Benchmark is currently running...")
 * @param {string} runHistory - Formatted run history entries
 * @returns {string} The complete formatted comment body
 */
function createCommentBody(status, runHistory) {
  return `## 📊 Benchmark Results\n\n${status}\n\n### Run History:\n${runHistory}\n\n<sub>Last updated: ${new Date().toISOString()}</sub>`;
}

/**
 * Creates a run entry for the history section.
 * @param {number} runNumber - The workflow run number
 * @param {string} owner - Repository owner
 * @param {string} repo - Repository name
 * @param {string} runId - The workflow run ID
 * @param {string} status - Status text for the run (e.g., "In Progress ⏳")
 * @returns {string} Formatted run entry line
 */
function createRunEntry(runNumber, owner, repo, runId, status) {
  return `- [Run #${runNumber}](https://github.com/${owner}/${repo}/actions/runs/${runId}) - ${status}`;
}

/**
 * Updates or creates the benchmark comment with "In Progress" status.
 * @param {import('@octokit/rest').Octokit} github - GitHub API client
 * @param {Object} context - GitHub Actions context
 */
async function postInitialComment(github, context) {
  const botComment = await findBenchmarkComment(github, context);
  const status = '⏳ Benchmark is currently running...';
  const newRunEntry = createRunEntry(
    context.runNumber,
    context.repo.owner,
    context.repo.repo,
    context.runId,
    'In Progress ⏳'
  );

  if (botComment) {
    // Extract existing history and append new run
    const existingHistory = extractRunHistory(botComment.body);
    const updatedHistory = existingHistory ? `${existingHistory}\n${newRunEntry}` : newRunEntry;
    const updatedBody = createCommentBody(status, updatedHistory);

    await github.rest.issues.updateComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      comment_id: botComment.id,
      body: updatedBody
    });
  } else {
    // Create new comment
    const body = createCommentBody(status, newRunEntry);
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      issue_number: context.issue.number,
      body: body
    });
  }
}

/**
 * Updates the benchmark comment with completion status and artifact link.
 * @param {import('@octokit/rest').Octokit} github - GitHub API client
 * @param {Object} context - GitHub Actions context
 * @param {string} artifactId - The artifact ID for the download link
 */
async function postCompletionComment(github, context, artifactId) {
  const artifactUrl = `https://github.com/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}/artifacts/${artifactId}`;
  const botComment = await findBenchmarkComment(github, context);
  const status = '✅ Benchmark completed! You can download the results from the links below.';

  if (botComment) {
    // Extract existing history
    const existingHistory = extractRunHistory(botComment.body);
    
    // Update the current run's status in history
    const runPattern = new RegExp(
      `- \\[Run #${context.runNumber}\\]\\(https://github\\.com/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}\\) - .*`
    );
    const newRunEntry = createRunEntry(
      context.runNumber,
      context.repo.owner,
      context.repo.repo,
      context.runId,
      `✅ Complete ([Download Artifact](${artifactUrl}))`
    );
    
    let updatedHistory;
    if (existingHistory.match(runPattern)) {
      // Update existing entry
      updatedHistory = existingHistory.replace(runPattern, newRunEntry);
    } else {
      // Append new entry (fallback if initial comment was missed)
      updatedHistory = existingHistory ? `${existingHistory}\n${newRunEntry}` : newRunEntry;
    }
    
    const body = createCommentBody(status, updatedHistory);

    await github.rest.issues.updateComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      comment_id: botComment.id,
      body: body
    });
  } else {
    // Create new comment if none exists (fallback)
    const newRunEntry = createRunEntry(
      context.runNumber,
      context.repo.owner,
      context.repo.repo,
      context.runId,
      `✅ Complete ([Download Artifact](${artifactUrl}))`
    );
    const body = createCommentBody(status, newRunEntry);
    
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      issue_number: context.issue.number,
      body: body
    });
  }
}

// Export functions for use in GitHub Actions workflow
module.exports = {
  findBenchmarkComment,
  extractRunHistory,
  createCommentBody,
  createRunEntry,
  postInitialComment,
  postCompletionComment
};
