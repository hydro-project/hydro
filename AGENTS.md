# Agent Guidelines

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <short summary>

<markdown body>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`.

For breaking changes, append `!` after the type (e.g. `refactor(hydro_lang)!:`), and add `BREAKING CHANGE: ...` footers
describing each breaking change (or summarizing if many).

## Descriptions

Write the description in **Markdown**. Use bullet lists, code spans, and headings to organize multi-topic changes. Example:

```
feat(hydro_lang): ...

... detailed explanation of key design decisions
```

## Comments

State the mechanism and the reason directly, as a causal chain: "X is the case, therefore Y, therefore Z."

- No defensive or apologetic framing. Do not justify a choice against alternatives not taken ("rather than X, which would…", "as it already was", "deliberately"), and do not narrate history ("earlier versions…"). If an alternative is a trap worth warning about, state the trap plainly.
- Lead with *why* the code exists; keep *how* short. Omit tangential facts, even true ones.
- Name the direct cause, not a chain of secondary effects (prefer "this key has the highest precedence" over describing which other keys get joined or ignored).
- A doc summary line says precisely what the item is or does.
- Every pronoun has an unambiguous referent in the same sentence.
- Be precise about distinct things with similar names (e.g. cargo's process environment vs. the environment it gives a build script).
- Delete comments whose code is gone.
