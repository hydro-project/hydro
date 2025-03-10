# `include_mdtests::include_mdtests!`

This macro allows you to include markdown files with Rust code blocks to be run as tests.

The macro takes one argument, a string path or [`glob`] pattern, **relative to the workspace root**.
The matched files will be included and converted to doc tests.

## Example

```rust,ignore
include_mdtests::include_mdtests!("my/markdown/test.md")
```
becomes
```rust,ignore
#[doc = include_str!("my/markdown/test.md")]
mod my_markdown_test_md {}
```
