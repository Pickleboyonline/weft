# Weft for VS Code

Syntax highlighting for `.weft` files, and for ```` ```weft ```` code blocks
inside Markdown files (in the editor; the Markdown *preview* uses its own
highlighter). The grammar tracks the draft language in
[docs/language.md](../../docs/language.md), so expect it to change with it.

## Install

From the repo root:

```bash
make vscode
```

That installs dependencies if needed, runs the grammar tests, packages the
extension and installs it. Run it again after changing the grammar. Reload VS
Code afterwards (**Developer: Reload Window**).

| Task | Command |
|---|---|
| Test, package and install | `make vscode` |
| Run the grammar tests | `make vscode-test` |
| Render a preview | `make vscode-preview`, or `make vscode-preview FILES=app.weft` |
| Package only | `make vscode-package` |
| Uninstall | `make vscode-uninstall` |

To try it without installing, open this folder in VS Code and press F5. That
starts a second VS Code window with the extension loaded.

## What gets highlighted

| Construct | Example | Scope |
|---|---|---|
| Declarations | `command`, `query`, `record`, `table`, `client`, `component`, `page`, `state`, `action`, `check`, `effect`, `handler`, `extern` | `storage.type.*` |
| The effect marker | the `!` in `insert!`, and `=>` | `keyword.other.effect` |
| Effect rows | `! <write todos, reject [EmptyTitle]>` | `meta.effects`, with `read`/`write`/`reject`/`random`/`time` as `keyword.other.effect.kind` |
| Control flow | `if`, `else`, `match`, `for`, `in`, `key`, `do`, `reject` | `keyword.control` |
| Markup elements | `li { … }`, `button(on_click: …)` | `entity.name.tag` |
| Types and tags | `Todo`, `Some(id)`, `EmptyTitle` | `entity.name.type`; built-ins (`Text`, `U64`, …) as `support.type` |
| Lambda parameters | `\|m, msg\|` | `variable.parameter` |
| Fields and named arguments | `{ draft: Text }`, `on_click:` | `variable.other.property` |
| Deployment vocabulary | `machine`, `place`, `consistency`, `budget`, `pin`, `session`, … | only inside `deploy`/`simulate` blocks |
| Units | `5ms`, `10%`, `$50/month` | `constant.numeric` with `keyword.other.unit` |

Scopes follow TextMate conventions, so any VS Code theme colors them.

## File icon

`.weft` files get a small weave: amber *weft* threads passing over and under
gray *warp* threads, in [icons/](icons/) with light- and dark-theme variants.

VS Code shows a language's own icon when the active file icon theme has none for
that language. That covers Seti (the default) and vscode-icons. A theme can opt
out by setting `showLanguageModeIcons: false`, and then `.weft` files get the
theme's generic file icon instead.

## Changing the grammar

The grammar is [syntaxes/weft.tmLanguage.json](syntaxes/weft.tmLanguage.json).
After editing it, run `make vscode-test`, and `make vscode-preview` to look at
the result in `out/preview.html`. Then `make vscode` to install the new version.

The tests do two things:
- checks specific tokens in real snippets against expected scopes;
- scans every `.weft` file in the repo and every ```` ```weft ```` block in the
  docs, failing if any non-space character isn't claimed by a rule.

New syntax in the docs that the grammar doesn't know about shows up there.
Both scripts use [Shiki](https://shiki.style), which tokenizes with
`vscode-textmate` (the engine VS Code uses) and VS Code's own Dark+ and Light+
themes.
