import TurndownService from 'turndown';

/**
 * Build a `<p data-diagram>` placeholder describing a diagram element.
 */
function createDiagramPlaceholder(alt: string): HTMLParagraphElement {
  const placeholder = document.createElement('p');
  placeholder.setAttribute('data-diagram', 'true');
  placeholder.textContent = alt ? `[Diagram: ${alt}]` : '[Diagram]';
  return placeholder;
}

/**
 * Replace diagram elements (SVG, canvas, mermaid) with placeholder
 * paragraphs. Turndown silently drops non-HTML elements like SVG without
 * invoking custom rules, so this must happen before conversion.
 */
function replaceDiagramsWithPlaceholders(html: string): string {
  const container = document.createElement('div');
  container.innerHTML = html;

  container.querySelectorAll('svg, canvas, .mermaid').forEach(el => {
    const alt =
      el.getAttribute('aria-label') ||
      el.getAttribute('title') ||
      el.closest('figure')?.querySelector('figcaption')?.textContent ||
      '';
    el.replaceWith(createDiagramPlaceholder(alt));
  });

  return container.innerHTML;
}

/**
 * Extract the language from Prism class names on a code block, checking
 * both the <code> and <pre> elements. Docusaurus may render classes like
 * "language-rust,ignore"; only the part before the comma is the language.
 */
function extractCodeLanguage(pre: HTMLElement, code: HTMLElement): string {
  const langClass =
    Array.from(code.classList).find(c => c.startsWith('language-')) ||
    Array.from(pre.classList).find(c => c.startsWith('language-'));
  return langClass ? langClass.replace('language-', '').split(',')[0] : '';
}

/**
 * Extract code text, preserving line breaks. Docusaurus/Prism wraps each
 * line in a `.token-line` span (line breaks are visual, via CSS or <br>),
 * so textContent alone would collapse everything onto one line.
 */
function extractCodeText(code: HTMLElement): string {
  const tokenLines = code.querySelectorAll('.token-line');
  if (tokenLines.length === 0) {
    return code.textContent || '';
  }
  return Array.from(tokenLines)
    .map(line => line.textContent || '')
    .join('\n');
}

/**
 * Convert rendered HTML from a Docusaurus doc page into Markdown.
 */
export function htmlToMarkdown(html: string): string {
  const preprocessed = replaceDiagramsWithPlaceholders(html);

  const turndown = new TurndownService({
    headingStyle: 'atx',
    codeBlockStyle: 'fenced',
    bulletListMarker: '-',
  });

  // Fenced code blocks with language preserved from Prism classes
  turndown.addRule('fencedCodeBlock', {
    filter(node) {
      return node.nodeName === 'PRE' && node.querySelector('code') !== null;
    },
    replacement(content, node) {
      const pre = node as HTMLElement;
      const code = pre.querySelector('code');
      if (!code) return content;
      const lang = extractCodeLanguage(pre, code);
      const text = extractCodeText(code);
      return `\n\n\`\`\`${lang}\n${text}\n\`\`\`\n\n`;
    },
  });

  // Docusaurus admonitions -> ::: syntax
  turndown.addRule('admonitions', {
    filter(node) {
      if (node.nodeName !== 'DIV') return false;
      const el = node as HTMLElement;
      return (
        el.getAttribute('role') === 'alert' ||
        Array.from(el.classList).some(c => c.includes('admonition'))
      );
    },
    replacement(content, node) {
      const el = node as HTMLElement;
      const type =
        el.dataset.admonitionType ||
        Array.from(el.classList)
          .find(c => /^admonition-\w+/.test(c))
          ?.replace('admonition-', '') ||
        'note';

      const title =
        el.querySelector('[class*="admonitionHeading"], .admonition-heading h5')
          ?.textContent || '';
      const bodyHtml =
        el.querySelector('[class*="admonitionContent"], .admonition-content')
          ?.innerHTML || '';

      const body = bodyHtml ? turndown.turndown(bodyHtml).trim() : content.trim();
      const titlePart = title && title.toLowerCase() !== type ? `[${title}]` : '';
      return `\n\n:::${type}${titlePart}\n${body}\n:::\n\n`;
    },
  });

  // Strip UI-only buttons (e.g. code block copy buttons)
  turndown.addRule('removeUiButtons', {
    filter(node) {
      return (
        node.nodeName === 'BUTTON' &&
        (node as HTMLElement).classList.contains('clean-btn')
      );
    },
    replacement: () => '',
  });

  // Diagram placeholders (from replaceDiagramsWithPlaceholders) -> italic text
  turndown.addRule('diagramPlaceholder', {
    filter(node) {
      return (
        node.nodeName === 'P' &&
        (node as HTMLElement).hasAttribute('data-diagram')
      );
    },
    replacement(_content, node) {
      const text = (node as HTMLElement).textContent || '[Diagram]';
      return `\n\n*${text}*\n\n`;
    },
  });

  // <figure> -> image with alt/caption, or diagram placeholder
  turndown.addRule('figures', {
    filter(node) {
      return node.nodeName === 'FIGURE';
    },
    replacement(_content, node) {
      const el = node as HTMLElement;
      const caption = el.querySelector('figcaption')?.textContent || '';
      const img = el.querySelector('img');
      if (img) {
        const alt = img.getAttribute('alt') || caption;
        return `\n\n![${alt}](${img.getAttribute('src') || ''})\n\n`;
      }
      return caption ? `\n\n*[Diagram: ${caption}]*\n\n` : '\n\n*[Diagram]*\n\n';
    },
  });

  return turndown.turndown(preprocessed);
}
