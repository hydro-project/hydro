import {describe, it, expect} from 'vitest';
import {htmlToMarkdown} from './htmlToMarkdown';

/** Extract the body of the first fenced code block for the given language. */
function codeBlock(md: string, lang: string): string[] {
  const match = md.match(new RegExp(`\`\`\`${lang}\n([\\s\\S]*?)\`\`\``));
  expect(match).not.toBeNull();
  return match![1].trim().split('\n');
}

describe('htmlToMarkdown', () => {
  describe('basic markdown', () => {
    it('converts paragraphs', () => {
      expect(htmlToMarkdown('<p>Hello world</p>')).toBe('Hello world');
    });

    it('converts headings to ATX style', () => {
      const md = htmlToMarkdown('<h1>Title</h1><h2>Subtitle</h2>');
      expect(md).toContain('# Title');
      expect(md).toContain('## Subtitle');
    });

    it('converts inline code', () => {
      expect(htmlToMarkdown('<p>Use <code>cargo build</code></p>')).toContain(
        '`cargo build`',
      );
    });

    it('converts links', () => {
      expect(
        htmlToMarkdown('<p><a href="/docs/reference">the reference</a></p>'),
      ).toContain('[the reference](/docs/reference)');
    });

    it('converts bullet lists', () => {
      const md = htmlToMarkdown('<ul><li>First</li><li>Second</li></ul>');
      expect(md).toMatch(/-\s+First/);
      expect(md).toMatch(/-\s+Second/);
    });
  });

  describe('code blocks', () => {
    it('extracts language from the <code> class', () => {
      const md = htmlToMarkdown(
        '<pre><code class="language-rust">fn main() {}</code></pre>',
      );
      expect(md).toContain('```rust\nfn main() {}\n```');
    });

    it('extracts language from the <pre> class when <code> has none', () => {
      const md = htmlToMarkdown(
        '<pre class="prism-code language-rust"><code class="codeBlockLines_e6Vv">fn main() {}</code></pre>',
      );
      expect(md).toContain('```rust');
    });

    it('strips metadata after comma in language (rust,ignore -> rust)', () => {
      const md = htmlToMarkdown(
        '<pre class="prism-code language-rust,ignore"><code>let x = 1;</code></pre>',
      );
      expect(md).toContain('```rust\n');
      expect(md).not.toContain('rust,ignore');
    });

    it('handles missing language', () => {
      expect(htmlToMarkdown('<pre><code>plain</code></pre>')).toContain(
        '```\nplain\n```',
      );
    });

    it('preserves line breaks across adjacent token-line spans', () => {
      // Prism separates lines visually (CSS display / <br>), not with \n
      const md = htmlToMarkdown(
        '<pre class="language-js"><code>' +
          '<span class="token-line">const x = 1;</span>' +
          '<span class="token-line">const y = 2;</span>' +
          '</code></pre>',
      );
      expect(codeBlock(md, 'js')).toEqual(['const x = 1;', 'const y = 2;']);
    });

    it('preserves line breaks with trailing <br> inside token-lines', () => {
      // Matches real Docusaurus output: each token-line ends with <br>
      const md = htmlToMarkdown(
        '<pre class="prism-code language-rust,ignore"><code class="codeBlockLines_e6Vv">' +
          '<span class="token-line"><span class="token keyword">let</span> a = 1;<br></span>' +
          '<span class="token-line"><span class="token keyword">let</span> b = 2;<br></span>' +
          '</code></pre>',
      );
      expect(codeBlock(md, 'rust')).toEqual(['let a = 1;', 'let b = 2;']);
    });

    it('removes copy buttons', () => {
      const md = htmlToMarkdown(
        '<pre><code>x</code></pre><button class="clean-btn">Copy</button>',
      );
      expect(md).not.toContain('Copy');
    });
  });

  describe('admonitions', () => {
    it('converts to ::: syntax with type and title', () => {
      const md = htmlToMarkdown(`
        <div role="alert" class="admonition" data-admonition-type="tip">
          <div class="admonitionHeading_xyz">Pro Tip</div>
          <div class="admonitionContent_xyz"><p>This is useful.</p></div>
        </div>
      `);
      expect(md).toContain(':::tip[Pro Tip]');
      expect(md).toContain('This is useful.');
    });
  });

  describe('diagrams', () => {
    it('replaces SVG with a placeholder, keeping surrounding content', () => {
      const md = htmlToMarkdown(
        '<p>Before.</p><svg viewBox="0 0 1 1"><rect/></svg><p>After.</p>',
      );
      expect(md).toContain('Before.');
      expect(md).toContain('*[Diagram]*');
      expect(md).toContain('After.');
    });

    it('uses aria-label as alt text', () => {
      const md = htmlToMarkdown('<svg aria-label="Data flow"><rect/></svg>');
      expect(md).toContain('*[Diagram: Data flow]*');
    });

    it('uses title as alt text on canvas', () => {
      const md = htmlToMarkdown('<canvas title="Perf graph"></canvas>');
      expect(md).toContain('*[Diagram: Perf graph]*');
    });

    it('replaces mermaid containers', () => {
      const md = htmlToMarkdown('<div class="mermaid"><svg><g/></svg></div>');
      expect(md).toBe('*[Diagram]*');
    });

    it('emits one placeholder per diagram', () => {
      const md = htmlToMarkdown('<svg><rect/></svg><svg><circle/></svg>');
      expect(md.match(/\*\[Diagram\]\*/g)).toHaveLength(2);
    });

    it('does not swallow content when a diagram is nested in wrapper divs', () => {
      // Regression: an over-greedy rule once reduced whole pages to [Diagram]
      const md = htmlToMarkdown(`
        <h1>Atomic Collections</h1>
        <div class="animation-container">
          <svg viewBox="0 0 1 1"><rect/></svg>
          <button class="play-btn">Play</button>
          <div class="progressBar"></div>
        </div>
        <p>The animation shows the problem.</p>
      `);
      expect(md).toContain('# Atomic Collections');
      expect(md).toContain('*[Diagram]*');
      expect(md).toContain('The animation shows the problem.');
    });
  });

  describe('figures', () => {
    it('converts figure with img to markdown image', () => {
      const md = htmlToMarkdown(`
        <figure>
          <img src="/img/arch.png" alt="System architecture" />
          <figcaption>Figure 1</figcaption>
        </figure>
      `);
      expect(md).toContain('![System architecture](/img/arch.png)');
    });

    it('uses figcaption as alt text for figure-wrapped diagrams', () => {
      const md = htmlToMarkdown(`
        <figure>
          <svg><rect/></svg>
          <figcaption>Flow diagram</figcaption>
        </figure>
      `);
      expect(md).toContain('*[Diagram: Flow diagram]*');
    });
  });
});
