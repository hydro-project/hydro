import React from 'react';
import { describe, it, expect } from 'vitest';
import { renderToString } from 'react-dom/server';
import { AntDockablePanel } from '../components/AntDockablePanel';

// Server-render-only smoke tests to avoid DOM/test-utils deps.
describe('AntDockablePanel (SSR smoke)', () => {
  it('renders title and children when open and not collapsed', () => {
    const html = renderToString(
      <AntDockablePanel title="Panel" defaultOpen={true} defaultCollapsed={false}>
        <div>content</div>
      </AntDockablePanel>
    );
    expect(html).toContain('Panel');
    expect(html).toContain('content');
  });

  it('omits children when defaultCollapsed=true', () => {
    const html = renderToString(
      <AntDockablePanel title="Panel" defaultOpen={true} defaultCollapsed={true}>
        <div>content</div>
      </AntDockablePanel>
    );
    expect(html).toContain('Panel');
    expect(html).not.toContain('content');
  });

  it('returns empty output when defaultOpen=false', () => {
    const html = renderToString(
      <AntDockablePanel title="Panel" defaultOpen={false}>
        <div>content</div>
      </AntDockablePanel>
    );
    expect(html).toBe('');
  });
});
