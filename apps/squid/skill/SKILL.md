---
name: squid
description: Author data-backed documents and reports by querying the user's SQLite, PostgreSQL, or MySQL database directly and by writing and rendering `.squid` files with the `squid` CLI. Use for generating Markdown or HTML from database content, building live SQL-backed reports, drafting narrative content from database records, or running Squid documents.
---

# Author with Squid

Treat `squid` as both a callable renderer and one part of a broader document-authoring workflow. Do not limit the work to invoking the CLI: connect to the user's database directly when inspecting data, analyzing results, or generating prose and other document content would produce a better result.

## Choose an authoring mode

- Use a `.squid` document when values should remain live and reproducible. Write Markdown around `{| SELECT ... |}` table blocks and `{ SELECT ... }` scalar blocks, then call `squid` to render it.
- Query the database directly when the document needs interpretation, synthesis, narrative writing, restructuring, or calculations that are clearer outside an embedded SQL block. Use those results to author the requested document yourself.
- Combine both approaches when useful: query directly to understand the data and write the narrative, while leaving selected tables or scalar values as live Squid blocks.

## Connect safely

- Read connection data from environment variables. Prefer `DATABASE_URL` because `squid` consumes it automatically when `--dbname` is omitted.
- Use the project's documented environment-variable names when it already separates host, port, database, username, and password.
- Never hardcode a host, port, database name, username, password, or connection URI in a `.squid` file, generated source, committed script, or example.
- Do not print, echo, log, or include connection secrets in generated documents. Avoid shell tracing while credentials are present.
- If required connection variables are absent, identify the missing variable names without inventing values.
- Prefer read-only credentials for discovery and document generation. Do not mutate the user's database unless the user explicitly requests a write.

For example, require the environment to provide the connection and let Squid read it:

```sh
: "${DATABASE_URL:?Set DATABASE_URL before rendering}"
squid report.squid --output report.md
```

When querying directly, use the matching database client or library and make it read the same environment configuration. Parameterize user-provided values instead of interpolating them into SQL.

## Author the document

1. Determine the requested audience, format, scope, and whether the output should be a live report or a snapshot.
2. Inspect the relevant schema and a small, representative result set directly from the database. Keep discovery queries read-only and bounded.
3. Draft the headings, explanations, conclusions, and layout. Ground every factual claim in query results and distinguish interpretation from stored facts.
4. Add dynamic Squid blocks where reproducibility matters:

   ```markdown
   # Account summary

   Active accounts: **{SELECT COUNT(*) FROM accounts WHERE active = TRUE}**

   {|SELECT plan, COUNT(*) AS accounts
   FROM accounts
   WHERE active = TRUE
   GROUP BY plan
   ORDER BY accounts DESC|}
   ```

5. Render `.squid` sources by actually calling `squid`; do not merely tell the user which command to run. Use `--output` for a requested artifact and select `.md`, `.html`, `.zip`, or `.tar.gz` through the output extension.
6. Inspect the rendered result for SQL errors, empty datasets, `NULL` values, accidental sensitive data, malformed Markdown, and unsupported database-dialect syntax. Revise and rerender as needed.

Keep generated content concise and useful. Include only the rows and fields needed for the user's purpose, especially when the database contains personal or confidential data.
