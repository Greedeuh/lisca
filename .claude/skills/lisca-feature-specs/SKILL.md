---
name: lisca-feature-specs
description: Guides writing feature specification files for the Lisca TTS app using the "As a user" scenario syntax.
---

# Writing Lisca Feature Specs

Feature specs live in `specs/` and describe user-facing behavior using scenario syntax.

## File Structure

Each spec file follows this template:

```markdown
# Feature Name

## Feature
One-paragraph description of what the feature does from the user's perspective.

## Scenarios

### Group Name (optional subsection)
- **As a user**, I can <action>, so <benefit>.
- **As a user**, when <trigger>, <expected outcome>.
- **As a user**, if <edge case>, <graceful behavior>.

## Key Files
- `path/to/file.rs` — role description
- `path/to/file.tsx` — role description
```

## Scenario Rules

1. **Start with "As a user"** — every scenario is a user story.
2. **Use "I can" for capabilities** — what the user is able to do.
3. **Use "when" for behaviors** — how the system reacts to user actions.
4. **Use "if" for edge cases** — error handling, empty states, missing data.
5. **Add "so" for benefits** — why the user wants this (optional but preferred).
6. **Group related scenarios** under `### Subsection` headers when a feature has distinct areas (e.g., "Queue Management" vs "Playback Controls").
7. **Include an Error Handling section** when the feature can fail (network errors, missing files, etc.).
8. **Include platform-specific behavior** when it differs (e.g., Windows vs Linux overlay).

## Key Files Section

- List every file that implements the feature.
- Format: `path` — one-line role description.
- Separate backend (Rust) from frontend (TypeScript) files.
- Include hook files, type definitions, and config files.

## What NOT to Include

- Internal implementation details (how ONNX inference works, rodio internals)
- Code snippets or API references
- Architecture decisions (those belong in `AGENTS.md` or memory)
- Performance characteristics unless user-visible (e.g., "loads in under 1s")

## Workflow

1. Read the relevant source files to understand the feature.
2. Write the spec file in `specs/` using the template above.
3. Add the new spec to `specs/README.md` index table.
4. Run `bun run build` to verify no frontend breakage.
