# Agent Guidelines

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <short summary>

<markdown body>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`.

For breaking changes, append `!` after the type (e.g. `refactor(infinity-daemon)!:`), and add `BREAKING CHANGE: ...` footers
describing each breaking change (or summarizing if many).

## Descriptions

Write the description in **Markdown**. Use bullet lists, code spans, and headings to organize multi-topic changes. Example:

```
feat(hydro_lang): ...

* Store sessions to `~/.infinity/sessions.json` on shutdown
* Restore on startup via `SessionStore::load()`
* Skip sessions older than 7 days
```
