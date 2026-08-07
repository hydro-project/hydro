import React, {type ReactNode} from 'react';
import DocBreadcrumbs from '@theme-original/DocBreadcrumbs';
import CopyMarkdownButton from '@site/src/components/CopyMarkdownButton';
import styles from './styles.module.css';

export default function DocBreadcrumbsWrapper(): ReactNode {
  return (
    <div className={styles.breadcrumbsRow}>
      <DocBreadcrumbs />
      <CopyMarkdownButton />
    </div>
  );
}
