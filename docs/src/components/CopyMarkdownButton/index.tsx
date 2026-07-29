import React, {useCallback, useState} from 'react';
import styles from './styles.module.css';

export default function CopyMarkdownButton(): React.ReactElement {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    // Find the article content on the page
    const article = document.querySelector('article');
    if (!article) return;

    // Lazily import to avoid SSR issues
    const {htmlToMarkdown} = await import('./htmlToMarkdown');

    // Clone to avoid modifying the DOM
    const clone = article.cloneNode(true) as HTMLElement;

    // Remove elements we don't want in the copied markdown
    // Remove breadcrumbs, footer, pagination, mobile TOC
    clone
      .querySelectorAll('.theme-doc-breadcrumbs, [class*="breadcrumbs"], .theme-doc-footer, .pagination-nav, [class*="tocMobile"]')
      .forEach(el => el.remove());
    // Remove our own copy button if it somehow appears in the article
    clone
      .querySelectorAll('[aria-label="Copy page as Markdown"]')
      .forEach(el => el.remove());

    const markdown = htmlToMarkdown(clone.innerHTML);
    const source = `\n\n---\nSource: ${window.location.href}\n`;
    await navigator.clipboard.writeText(markdown + source);

    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, []);

  return (
    <button
      className={styles.copyButton}
      onClick={handleCopy}
      title="Copy page as Markdown"
      aria-label="Copy page as Markdown">
      {copied ? (
        <>
          <CheckIcon />
          <span className={styles.label}>Copied!</span>
        </>
      ) : (
        <>
          <CopyIcon />
          <span className={styles.label}>Copy as Markdown</span>
        </>
      )}
    </button>
  );
}

function CopyIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true">
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}
