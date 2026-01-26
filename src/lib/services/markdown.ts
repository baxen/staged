/**
 * Simple markdown renderer for chat messages.
 * Supports: bold, italic, inline code, code blocks, links, and paragraphs.
 */

/**
 * Escape HTML entities to prevent XSS.
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

/**
 * Render inline markdown (bold, italic, code, links).
 */
function renderInline(text: string): string {
  let result = escapeHtml(text);

  // Inline code (must come before bold/italic to avoid conflicts)
  result = result.replace(/`([^`]+)`/g, '<code>$1</code>');

  // Bold: **text** or __text__
  result = result.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  result = result.replace(/__([^_]+)__/g, '<strong>$1</strong>');

  // Italic: *text* or _text_ (but not inside words for underscore)
  result = result.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  result = result.replace(/(?<!\w)_([^_]+)_(?!\w)/g, '<em>$1</em>');

  // Links: [text](url)
  result = result.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
  );

  // Auto-link URLs
  result = result.replace(
    /(?<!")https?:\/\/[^\s<]+/g,
    '<a href="$&" target="_blank" rel="noopener noreferrer">$&</a>'
  );

  return result;
}

/**
 * Render markdown text to HTML.
 */
export function renderMarkdown(text: string): string {
  const lines = text.split('\n');
  const result: string[] = [];
  let inCodeBlock = false;
  let codeBlockContent: string[] = [];
  let codeBlockLang = '';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Code block start/end
    if (line.startsWith('```')) {
      if (inCodeBlock) {
        // End code block
        result.push(
          `<pre><code class="language-${codeBlockLang}">${escapeHtml(codeBlockContent.join('\n'))}</code></pre>`
        );
        codeBlockContent = [];
        codeBlockLang = '';
        inCodeBlock = false;
      } else {
        // Start code block
        codeBlockLang = line.slice(3).trim();
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeBlockContent.push(line);
      continue;
    }

    // Empty line
    if (line.trim() === '') {
      continue;
    }

    // Headers
    const headerMatch = line.match(/^(#{1,6})\s+(.+)$/);
    if (headerMatch) {
      const level = headerMatch[1].length;
      result.push(`<h${level}>${renderInline(headerMatch[2])}</h${level}>`);
      continue;
    }

    // Unordered list items
    if (line.match(/^[-*+]\s+/)) {
      const content = line.replace(/^[-*+]\s+/, '');
      result.push(`<li>${renderInline(content)}</li>`);
      continue;
    }

    // Ordered list items
    if (line.match(/^\d+\.\s+/)) {
      const content = line.replace(/^\d+\.\s+/, '');
      result.push(`<li>${renderInline(content)}</li>`);
      continue;
    }

    // Regular paragraph
    result.push(`<p>${renderInline(line)}</p>`);
  }

  // Close unclosed code block
  if (inCodeBlock && codeBlockContent.length > 0) {
    result.push(
      `<pre><code class="language-${codeBlockLang}">${escapeHtml(codeBlockContent.join('\n'))}</code></pre>`
    );
  }

  // Wrap consecutive list items in ul
  let html = result.join('\n');
  html = html.replace(/(<li>.*?<\/li>\n?)+/g, (match) => `<ul>${match}</ul>`);

  return html;
}
