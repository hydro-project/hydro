import React, {useCallback, useState} from 'react';
import {useDoc} from '@docusaurus/plugin-content-docs/client';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {rawSourceUrl} from './rawSourceUrl';
import styles from './styles.module.css';

type CopyState = 'idle' | 'copied' | 'error';

export default function CopyMarkdownButton(): React.ReactElement | null {
  const {metadata} = useDoc();
  const rawUrl = useBaseUrl(rawSourceUrl(metadata.source) ?? '');
  const [state, setState] = useState<CopyState>('idle');

  const handleCopy = useCallback(async () => {
    try {
      const response = await fetch(rawUrl);
      if (!response.ok) {
        throw new Error(`Fetching ${rawUrl} failed: ${response.status}`);
      }
      const markdown = await response.text();
      const source = `\n\n---\nSource: ${window.location.href}\n`;
      await navigator.clipboard.writeText(markdown + source);
      setState('copied');
    } catch (error) {
      console.error('Copy as Markdown failed:', error);
      setState('error');
    }
    setTimeout(() => setState('idle'), 2000);
  }, [rawUrl]);

  // Pages without a markdown source (e.g. generated pages) get no button
  if (!rawSourceUrl(metadata.source)) {
    return null;
  }

  return (
    <button
      className={styles.copyButton}
      onClick={handleCopy}
      title="Copy page as Markdown"
      aria-label="Copy page as Markdown">
      {state === 'copied' && (
        <>
          <CheckIcon />
          <span className={styles.label}>Copied!</span>
        </>
      )}
      {state === 'error' && <span className={styles.label}>Copy failed</span>}
      {state === 'idle' && (
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
