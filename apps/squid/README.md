# SQUID

SQUID, short for SQL to UI Markdown, allows you to author Markdown content with executable SQL.
It executes each SQL block against SQLite and replaces the block with Markdown
tables or plaintext values, while leaving the surrounding headings and
explanation intact.

## Install

Install Squid through Dome:

```console
dome add squid
```

Or use the standalone installer from this monorepo:

```console
curl -fsSL https://raw.githubusercontent.com/bathan1/dome/main/apps/squid/scripts/install.sh | sh
```

Or install from crates.io:

```console
cargo install squidown --version 0.2.0-alpha.1
```

## Quick start

Build the CLI:

```console
cargo build -p squidown --release
```

Create a SQUID file such as [`examples/report.squid`](./examples/report.squid):

```markdown
# Benchmark report

## Average runtimes
The average across *all* trials.

{|SELECT
    ROUND(AVG(time_us_blue3)) || 'μs' AS blue3,
    ROUND(AVG(time_us_z3)) || 'μs' AS z3
FROM benchmarks|}

## Blue3-only count
How many formulas were solved by `blue3` alone?

{|SELECT COUNT(DISTINCT formula_id) AS count
FROM benchmarks
WHERE was_backend_used = 'false'|}
```

Render it using an existing SQLite database:

```console
target/release/squid report.squid --dbname benchmarks.sqlite
```

The rendered Markdown is written to standard output by default. Use `--output`
to write it directly to a file:

```console
target/release/squid report.squid \
  --dbname benchmarks.sqlite \
  --output benchmark-report.md
```

During development, the equivalent command is:

```console
cargo run -p squidown -- report.squid --dbname benchmarks.sqlite
```

Use the default in-memory database when the document creates and populates its
own tables:

```console
cargo run -p squidown -- report.squid
```

## SQL blocks

Use `{| ... |}` blocks to render query results as Markdown tables. They may
occupy one line:

```markdown
{|SELECT COUNT(*) AS cases FROM benchmarks|}
```

Or multiple lines:

```markdown
{|
SELECT formula, time_us
FROM benchmarks
ORDER BY time_us DESC
LIMIT 5
|}
```

Use `{ ... }` blocks to render a plaintext scalar value:

```markdown
Solved by **{SELECT username FROM users WHERE user_id = 1}**.
```

Plaintext blocks must return exactly one column and one row. If you need to
format multiple values as plaintext, aggregate them in SQL first.

Blocks execute in document order, so earlier table blocks can create tables or
insert data used by later queries. Statements that do not return rows produce a
short message containing the number of changed rows.

SQLite shell commands such as `.headers`, `.mode`, and `.print` are not needed:
Markdown supplies the report structure, and `squid` supplies the table formatting.

## Pretty tables

By default, `squid` aligns table columns in the Markdown source:

```markdown
| id  | name |
| --- | ---- |
| 1   | Ada  |
```

`--compact` is a valueless flag that disables column alignment:

```console
squid report.squid --database data.sqlite --compact
```

```markdown
| id | name |
| --- | --- |
| 1 | Ada |
```

Both forms render as the same table in a Markdown viewer.

## Init SQL

Use `--init` to execute a SQL file before rendering. If `--database` / `-d` is
omitted, the init file runs against the default in-memory SQLite database:

```console
squid report.squid --init setup.sql
squid report.squid --database data.sqlite --init setup.sql
```

## HTML output

If no `--output` file is specified, output is written to stdout as Markdown. Setting the output file
extension to `.md` will render Markdown, so by default, `squid` outputs plain markdown.

You can set `--output` file extension to `.html` and it will output a HTML view:

```console
squid report.squid --database data.sqlite -o report.html
```

Use `-`, `-.md`, or `-.html` as the output path to write that rendered view to stdout:

```console
squid report.squid --database data.sqlite -o -.html
```

## Multi-file output

Pass more than one input file to render multiple pages into an archive. Multi-file
rendering requires `--output` with a `.zip` or `.tar.gz` extension:

```console
squid index.squid users.squid benchmarks.squid --database data.sqlite -o content.zip
```

Archive entries default to Markdown. Use `.html.zip` or `.html.tar.gz` to render
each page as HTML:

```console
squid index.squid users.squid benchmarks.squid --database data.sqlite -o content.html.zip
squid index.squid users.squid benchmarks.squid --database data.sqlite -o content.html.tar.gz
```

Shell-expanded globs work as input lists. Recursive globs preserve the directory
structure under the common input directory:

```console
squid examples/multi-page/*.squid --database examples/.db -o content.zip
squid examples/multi-page/**/*.squid --database examples/.db -o content.html.tar.gz
```

## Command line

```text
squid [OPTIONS] <INPUT>...

Arguments:
  <INPUT>...  SQUID files containing brace-delimited SQL blocks

Options:
  -d, --dbname <DBNAME>      Database name, local SQLite path, or full postgresql://, postgres://, or mysql:// URI
      --database <DBNAME>    Alias for --dbname
  -i, --init <INIT>          Execute this SQL file against the database before rendering
  -h, --host <HOST>          Database host for Postgres/MySQL connections
  -p, --port <PORT>          Database port for Postgres/MySQL connections
  -U, --username <USERNAME>  Database username for Postgres/MySQL connections
      --password <PASSWORD>  Database password for Postgres/MySQL connections
      --driver <DRIVER>      Force the database driver when flags are ambiguous [possible values: sqlite, postgres, mysql]
  -o, --output <OUTPUT>      Write output path; use -, -.md, -.html for stdout or .zip, .tar.gz for archives
      --compact              Do not align generated Markdown table columns
      --help                 Print help
  -V, --version              Print version
```

When `--dbname` is omitted, squid uses `DATABASE_URL` if it is set, then falls
back to an in-memory SQLite database. When `--dbname` is a file path, squid opens
it as a local SQLite database. When psql-style connection flags are present,
squid assumes Postgres unless `--driver mysql` is provided:

```console
squid report.squid -h localhost -p 5432 -U alice -d benchmarks
squid report.squid -d "postgresql://alice:secret@localhost:5432/benchmarks"
squid report.squid "mysql://alice:secret@localhost:3306/benchmarks"
```

## Development

```console
cargo test -p squidown
cargo fmt --all --check
cargo clippy -p squidown --all-targets -- -D warnings
```

The `squidown` package exposes the library API as well as the `squid` binary.

## Editor support

Syntax highlighting for `.squid` files is available for VS Code and Neovim under
[`editors/`](editors/README.md). Both integrations use the editor's existing
Markdown and SQL highlighters.

## Motivation

The project stems from the benchmark scripts I created for my master's research on JHU's [caprice-lang](https://github.com/JHU-PL-Lab/caprice-lang),
where my primary goal was to speed up its SMT solver. My PI wanted benchmarks
on my runtimes to show that my performance boosts were significant (enough).
So I ran timed trials of my solver and recorded them into an SQLite database:

```ocaml
let time_us_blue3 =
  find_avg "blue3" results
in

let time_us_z3 =
  find_avg "z3" results
in

let was_backend_used =
  !metadata_ref.Solve.was_backend_used
in

Printf.eprintf
  "INSERT INTO benchmarks (formula_id, formula, was_backend_used,\
  time_us_blue3, time_us_z3) VALUES (%d, '%s', '%s', %.6f, %.6f);\n"
  formula_id
  formula_sql
  (Bool.to_string was_backend_used)
  time_us_blue3
  time_us_z3)
```

Then I would present a dump of the `benchmarks` table like so:

```sql
sqlite> SELECT * FROM BENCHMARKS LIMIT 5;
| formula_id |              formula               | was_backend_used | time_us_blue3 | time_us_z3 |
|------------|------------------------------------|------------------|---------------|------------|
| 0          | (0 < a) ^ ((b + a) < b)            | true             | 106.739044    | 107.204914 |
| 1          | (b <= a) ^ (a <= c)                | false            | 4.687071      | 204.385996 |
| 2          | (0 <= a) ^ (0 < b) ^ ((a + b) < 0) | true             | 130.391121    | 126.899958 |
| 3          | (b <= a) ^ (c <= b) ^ (a < c)      | false            | 4.958153      | 140.368223 |
| 4          | (0 <= a) ^ (0 < a)                 | false            | 2.012253      | 167.756796 |
```

The raw data-dump gave us a general idea on performance but we wanted to get some averages.
So I would start executing aggregate queries on the fly in lab:

```sql
SELECT ROUND(AVG(time_us_blue3)) || 'μs' as avg FROM benchmarks;
|   avg   |
|---------|
| 231.0μs |
```

I got tired of that so I decided to prewrite the relevant aggregate queries I wanted to present in lab.
Since I was using SQLite which comes with a Markdown formatter, I decided to encode both the narrative content
and the queries in my query script:

```sql
.headers on
.mode markdown

.print "# Benchmark report"

.print ""
.print "## Average runtimes"
.print "The average across *all* trials."
.print ""

SELECT
    ROUND(AVG(time_us_blue3)) || 'µs' AS blue3,
    ROUND(AVG(time_us_z3)) || 'µs' AS z3
FROM benchmarks;

.print ""
.print "## Blue3-only count"
.print "How many formulas were solved by `blue3` alone?"

SELECT COUNT(DISTINCT formula_id) AS count
FROM benchmarks
WHERE was_backend_used = 'false';```

This outputs:

```md
# Benchmark report

## Average runtimes
The average across *all* trials.

|  blue3  |   z3    |
|---------|---------|
| 218.0µs | 325.0µs |

## Blue3-only count
How many formulas were solved by `blue3` alone?

| count |
|-------|
| 89    |
```

The output was good enough that I was done as far as my benchmarking needs,
but as I added more and more queries, writing out the `.print` directives to format content
got tedious. So I wrapped all the markdown formatting logic in `squid` so it would be easier
to author these benchmarks, and now, we're not tied to just SQLite for analysis.
