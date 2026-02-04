/**
 * Shared utilities for managing benchmark results comments in GitHub PRs.
 * 
 * This module provides functions to find, extract, and update benchmark
 * results in PR comments, maintaining a history of all benchmark runs.
 */

/**
 * Updates or creates the benchmark comment.
 * @param {import('@octokit/rest').Octokit} github - GitHub API client
 * @param {Object} context - GitHub Actions context
 * @param {string} [artifactId] - The artifact ID for the download link (if provided, marks as completion)
 */
async function postBenchmarkComment(github, context, artifactId) {
  const isCompletion = artifactId !== undefined;
  
  // Find existing benchmark results comment in the PR
  const { data: comments } = await github.rest.issues.listComments({
    owner: context.repo.owner,
    repo: context.repo.repo,
    issue_number: context.issue.number,
  });
  const botComment = comments.find(comment =>
    comment.user.type === 'Bot' && comment.body.includes('📊 Benchmark Results')
  ) || null;
  
  // Determine status and run entry based on whether this is initial or completion
  let status, runStatus;
  if (isCompletion) {
    const artifactUrl = `https://github.com/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}/artifacts/${artifactId}`;
    status = '✅ Benchmark completed! You can download the results from the links below.';
    runStatus = `✅ Complete ([Download Artifact](${artifactUrl}))`;
  } else {
    status = '⏳ Benchmark is currently running...';
    runStatus = 'In Progress ⏳';
  }
  
  // Format the run entry line
  const newRunEntry = `- [Run #${context.runNumber}](https://github.com/${context.repo.owner}/${context.repo.repo}/actions/runs/${context.runId}) - ${runStatus}`;

  if (botComment) {
    // Extract existing history from comment body (between "### Run History:" and timestamp)
    const historyMatch = botComment.body.match(/### Run History:\n([\s\S]*?)\n\n<sub>/);
    const existingHistory = historyMatch ? historyMatch[1] : '';
    
    let updatedHistory;
    if (isCompletion && existingHistory) {
      // Try to update existing entry for this run
      // Escape special regex characters in user-controlled values to prevent regex injection
      const escapeRegex = (str) => String(str).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const escapedOwner = escapeRegex(context.repo.owner);
      const escapedRepo = escapeRegex(context.repo.repo);
      const escapedRunNumber = escapeRegex(context.runNumber);
      const escapedRunId = escapeRegex(context.runId);
      
      const runPattern = new RegExp(
        `- \\[Run #${escapedRunNumber}\\]\\(https://github\\.com/${escapedOwner}/${escapedRepo}/actions/runs/${escapedRunId}\\) - .*`
      );
      
      if (existingHistory.match(runPattern)) {
        // Update existing entry
        updatedHistory = existingHistory.replace(runPattern, newRunEntry);
      } else {
        // Append new entry (fallback if initial comment was missed)
        updatedHistory = `${existingHistory}\n${newRunEntry}`;
      }
    } else {
      // Append new run entry for initial comment
      updatedHistory = existingHistory ? `${existingHistory}\n${newRunEntry}` : newRunEntry;
    }
    
    // Format the complete comment body with status and run history
    const updatedBody = `## 📊 Benchmark Results\n\n${status}\n\n### Run History:\n${updatedHistory}\n\n<sub>Last updated: ${new Date().toISOString()}</sub>`;

    await github.rest.issues.updateComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      comment_id: botComment.id,
      body: updatedBody
    });
  } else {
    // Create new comment if none exists
    const body = `## 📊 Benchmark Results\n\n${status}\n\n### Run History:\n${newRunEntry}\n\n<sub>Last updated: ${new Date().toISOString()}</sub>`;
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      issue_number: context.issue.number,
      body: body
    });
  }
}

// Export function for use in GitHub Actions workflow
module.exports = {
  postBenchmarkComment
};
