---
name: changelog-entry
description: Add a numbered decision changelog entry and link it from README
---

Use when the user asks for a changelog / decision record, or after a meaningful architectural or UX change.

## Steps

1. **Find the next number** — List `docs/changelog/` and pick the next 4-digit prefix (e.g. if `0028-*.md` is latest, use `0029-`).

2. **Create** `docs/changelog/NNNN-short-kebab-title.md` modeled on existing entries:
   - Title: `# Decision NNNN: …`
   - Sections such as **Context**, **Decision**, **Key Considerations** (match the style of neighboring files like `0028-client-ux-improvements.md`).

3. **Update README** — In `README.md`, section **Living Lab Manual (Changelog)**, add a bullet link to the new file:
   - `[NNNN-short-title.md](./docs/changelog/NNNN-short-title.md)`

4. **Do not** duplicate the full changelog body in README; only add the one-line link.
