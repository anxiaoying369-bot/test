import DOMPurify from 'dompurify';
import { marked } from 'marked';

/**
 * Render untrusted Markdown as sanitized HTML.
 * Use this for all v-html Markdown sinks; never pass marked() output directly.
 */
export function renderSafeMarkdown(content: string): string {
  const html = marked.parse(content || '', {
    async: false,
    breaks: true,
    gfm: true,
  }) as string;

  return DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    ADD_ATTR: ['target', 'rel'],
  });
}
