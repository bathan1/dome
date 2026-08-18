# SQUID editor highlighting

Both integrations treat an SQUID document as Markdown and delegate
brace-delimited blocks to the editor's installed SQL highlighter. They do not
define their own Markdown, SQL, or color rules.

An SQL block starts when `{` is the first non-whitespace character on a line
and ends when `}` is the final non-whitespace character on a line:

```squid
# Markdown heading

{SELECT COUNT(*) AS count FROM benchmarks}

{
SELECT formula, time_us
FROM benchmarks
ORDER BY time_us DESC
}
```

Inline braces such as `text {like this}` remain Markdown.

## VS Code

The extension composes VS Code's registered `text.html.markdown` and
`source.sql` TextMate grammars. Open `editors/vscode` as an Extension
Development Host, or package it with:

```console
cd editors/vscode
npx @vscode/vsce package
code --install-extension squid-language-0.1.0.vsix
```

An installed SQL extension can supply or augment `source.sql`; no SQL colors
are hard-coded here.

## Neovim

The repository includes a root-level loader for lazy.nvim. Install the GitHub
repository as a normal plugin:

```lua
{
  "bathan1/dome",
  name = "squid",
  lazy = false,
}
```

After pushing changes to GitHub, run `:Lazy sync` and reopen Neovim. Keeping
`lazy = false` ensures `.squid` filetype detection is registered before files are
opened.

For a local checkout instead:

```lua
{
  dir = "/path/to/dome/apps/squid",
  lazy = false,
}
```

The adapter loads the user's existing `syntax/markdown.vim` and
`syntax/sql.vim`, then gives the SQL region precedence. This is intentional:
Neovim cannot use a Markdown Tree-sitter parser and an SQL Tree-sitter parser
for custom regions without a dedicated Tree-sitter container parser. The
syntax adapter composes the installed editor highlighters without maintaining
a second Markdown or SQL grammar.
