import { describe, it, expect } from 'vitest';
import { parseFrontmatter } from '../../packages/core/dist/frontmatter.js';

describe('parseFrontmatter: YAML multiline list support (H2 fix)', () => {
  it('parses YAML multiline list format for tags', () => {
    const text = `---
id: test-id
title: Test Entry
tags:
  - friction
  - testing
  - bucket:test
---

Body content here`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toEqual(['friction', 'testing', 'bucket:test']);
    expect(Array.isArray(result.meta.tags)).toBe(true);
  });

  it('parses JSON array format for tags', () => {
    const text = `---
id: test-id
tags: ["friction", "testing"]
---

Body`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toEqual(['friction', 'testing']);
  });

  it('handles quoted items in YAML multiline list', () => {
    const text = `---
id: test-id
tags:
  - "friction"
  - 'testing'
  - bucket:test
---

Body`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toEqual(['friction', 'testing', 'bucket:test']);
  });

  it('handles empty YAML list gracefully', () => {
    const text = `---
id: test-id
tags:
---

Body`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toBe('');
  });

  it('stops parsing list items at first non-dash line', () => {
    const text = `---
id: test-id
tags:
  - friction
  - testing
kind: insight
---

Body`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toEqual(['friction', 'testing']);
    expect(result.meta.kind).toBe('insight');
  });

  it('handles indentation variation in YAML lists', () => {
    const text = `---
id: test-id
tags:
  - friction
   - deeply-indented-item
  - testing
---

Body`;

    const result = parseFrontmatter(text);
    // Our parser is lenient and accepts any leading whitespace before the dash
    expect(result.meta.tags).toEqual(['friction', 'deeply-indented-item', 'testing']);
  });

  it('correctly extracts body after YAML multiline lists', () => {
    const text = `---
id: test-id
tags:
  - friction
---

This is the body
With multiple lines
And content`;

    const result = parseFrontmatter(text);
    expect(result.body).toBe('This is the body\nWith multiple lines\nAnd content');
  });

  it('regression test: empty tags field from Apr 13-19 2026 files', () => {
    // Real example from vault
    const text = `---
id: 01KP4DDVJ61CDSWPZFN6JKG5WX
title: "Friction 5/10: unknown (26m)"
kind: insight
category: knowledge
tags:
  - friction
  - flywheel
  - session-score
  - bucket:unknown
tier: working
source: friction-capture
created: 2026-04-13T21:54:40.000Z
---

## Session: cb269f35-62c
- **CWD:** unknown
- **Duration:** 26m
- **Tools:** 38`;

    const result = parseFrontmatter(text);
    expect(result.meta.tags).toEqual(['friction', 'flywheel', 'session-score', 'bucket:unknown']);
    expect(result.meta.id).toBe('01KP4DDVJ61CDSWPZFN6JKG5WX');
  });
});
