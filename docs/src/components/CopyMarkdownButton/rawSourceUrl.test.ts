import {describe, it, expect} from 'vitest';
import {rawSourceUrl} from './rawSourceUrl';

describe('rawSourceUrl', () => {
  it('maps @site docs sources to the raw URL', () => {
    expect(rawSourceUrl('@site/docs/hydro/learn/quickstart.mdx')).toBe(
      '/raw/docs/hydro/learn/quickstart.mdx',
    );
  });

  it('supports .md files', () => {
    expect(rawSourceUrl('@site/docs/dfir/architecture/handoffs.md')).toBe(
      '/raw/docs/dfir/architecture/handoffs.md',
    );
  });

  it('returns null for sources outside the docs directory', () => {
    expect(rawSourceUrl('@site/src/pages/research.mdx')).toBeNull();
  });

  it('returns null for non-markdown sources', () => {
    expect(rawSourceUrl('@site/docs/hydro/SomeAnimation.js')).toBeNull();
  });
});
