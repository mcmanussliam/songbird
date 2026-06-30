# CLAUDE.md

@AGENTS.md

Everything below is Claude Code–specific and layers on top of the shared instructions above.

## Subagents

For milestone-scale work, consider defining subagents in `.claude/agents/` scoped per crate or
layer (e.g. a `recurrence-engine` subagent that only touches `core/songbird-recurrence/`, a
`flutter-ui` subagent that only touches `app/lib/presentation/`) once the codebase is large
enough that context-window isolation between them is actually useful. Not needed yet at M1 — one
focused session per crate is enough while the core is still small.

## Skills

If recurring multi-step procedures emerge (e.g. "how to add a new conformance fixture," "how to
cut a release") that don't belong in AGENTS.md as standing rules, package them as a skill under
`.claude/skills/` rather than growing AGENTS.md indefinitely.

## Personal/local overrides

Machine-specific paths, personal shortcuts, or anything that shouldn't be committed for the whole
team belongs in `CLAUDE.local.md` (gitignored), not here.
