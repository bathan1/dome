//! Core parsing and SQL execution for SQUID documents.

mod html;

use std::path::Path;

use anyhow::{Context, Result, bail};
use mysql::prelude::Queryable;
use rusqlite::{Connection, types::Value};

pub use html::{render_github_html, render_github_html_named};

/// A database connection that can execute SQUID SQL blocks.
pub trait SqlExecutor {
    /// Execute SQL and return either a result set or the affected row count.
    fn execute_sql(&mut self, sql: &str) -> Result<QueryOutput>;
}

/// Normalized SQL block output from supported database drivers.
pub enum QueryOutput {
    /// A statement that did not return columns.
    Statement { changed: u64 },
    /// A result set with column headings and string-rendered cells.
    Rows {
        headings: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// Render Markdown by executing every brace-delimited SQL block against SQLite.
///
/// Table SQL blocks use `{|select 1|}` and render Markdown tables. Scalar SQL
/// blocks use `{select 1}` and render one-column, one-row results as text.
pub fn render_compact(markdown: &str, database: &Path) -> Result<String> {
    render(markdown, database, false)
}

/// Render Markdown using an existing SQLite connection.
pub fn render_compact_with_connection(markdown: &str, connection: &Connection) -> Result<String> {
    let mut connection = connection;
    render_with_executor(markdown, &mut connection, false)
}

/// Render Markdown with generated table columns aligned for readability.
pub fn render_pretty(markdown: &str, database: &Path) -> Result<String> {
    render(markdown, database, true)
}

/// Render Markdown with generated table columns aligned using an existing connection.
pub fn render_pretty_with_connection(markdown: &str, connection: &Connection) -> Result<String> {
    let mut connection = connection;
    render_with_executor(markdown, &mut connection, true)
}

/// Render Markdown using an existing supported SQL executor.
pub fn render_compact_with_executor(
    markdown: &str,
    executor: &mut impl SqlExecutor,
) -> Result<String> {
    render_with_executor(markdown, executor, false)
}

/// Render Markdown with generated table columns aligned using a supported SQL executor.
pub fn render_pretty_with_executor(
    markdown: &str,
    executor: &mut impl SqlExecutor,
) -> Result<String> {
    render_with_executor(markdown, executor, true)
}

fn render(markdown: &str, database: &Path, pretty: bool) -> Result<String> {
    let connection = Connection::open(database)
        .with_context(|| format!("failed to open SQLite database {}", database.display()))?;
    render_with_connection(markdown, &connection, pretty)
}

fn render_with_connection(markdown: &str, connection: &Connection, pretty: bool) -> Result<String> {
    let mut connection = connection;
    render_with_executor(markdown, &mut connection, pretty)
}

fn render_with_executor(
    markdown: &str,
    executor: &mut impl SqlExecutor,
    pretty: bool,
) -> Result<String> {
    let mut output = String::new();
    let mut lines = markdown.lines().enumerate();
    let mut fence: Option<(char, usize)> = None;

    while let Some((line_index, line)) = lines.next() {
        let trimmed = line.trim();
        if let Some((marker, length)) = fence {
            push_line(&mut output, line);
            if fence_marker(trimmed).is_some_and(|(candidate, candidate_length)| {
                candidate == marker && candidate_length >= length
            }) {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            push_line(&mut output, line);
            continue;
        }
        if !trimmed.starts_with('{') || !starts_standalone_block(trimmed) {
            let (rendered_inline, rendered) = render_inline_scalar_queries(line, executor)?;
            if rendered {
                output.push_str(&rendered_inline);
                if !rendered_inline.ends_with('\n') {
                    output.push('\n');
                }
                continue;
            }
            push_line(&mut output, line);
            continue;
        }

        let (kind, mut sql) = if let Some(table_sql) = trimmed.strip_prefix("{|") {
            (SqlBlockKind::Table, table_sql.trim_end().to_owned())
        } else {
            (SqlBlockKind::Scalar, trimmed[1..].trim_end().to_owned())
        };
        if let Some(single_line_sql) = strip_block_suffix(&sql, kind) {
            output.push_str(&execute_block(
                executor,
                single_line_sql.trim(),
                kind,
                pretty,
            )?);
            continue;
        }

        let mut closed = false;
        for (_, sql_line) in lines.by_ref() {
            let trimmed_sql_line = sql_line.trim_end();
            if let Some(final_sql) = strip_block_suffix(trimmed_sql_line, kind) {
                if !final_sql.is_empty() {
                    sql.push('\n');
                    sql.push_str(final_sql);
                }
                closed = true;
                break;
            }
            sql.push('\n');
            sql.push_str(sql_line);
        }

        if !closed {
            bail!(
                "unclosed SQL block starting at line {}\n\n{}",
                line_index + 1,
                source_excerpt(markdown, line_index)
            );
        }
        output.push_str(&execute_block(executor, sql.trim(), kind, pretty)?);
    }

    Ok(output)
}

#[derive(Clone, Copy)]
enum SqlBlockKind {
    Scalar,
    Table,
}

fn strip_block_suffix(sql: &str, kind: SqlBlockKind) -> Option<&str> {
    match kind {
        SqlBlockKind::Scalar => sql.strip_suffix('}'),
        SqlBlockKind::Table => sql.strip_suffix("|}"),
    }
}

fn starts_standalone_block(line: &str) -> bool {
    if line.starts_with("{|") {
        return true;
    }

    let Some(after_open) = line.strip_prefix('{') else {
        return false;
    };
    match after_open.find('}') {
        Some(close) => close == after_open.len() - 1,
        None => true,
    }
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn render_inline_scalar_queries(
    line: &str,
    executor: &mut impl SqlExecutor,
) -> Result<(String, bool)> {
    let mut output = String::new();
    let mut remainder = line;
    let mut rendered = false;

    while let Some((open, close)) = find_inline_scalar_span(remainder) {
        let sql = remainder[open + 1..close].trim();
        output.push_str(&remainder[..open]);
        output.push_str(&execute_scalar(executor, sql)?);
        remainder = &remainder[close + 1..];
        rendered = true;
    }

    output.push_str(remainder);
    Ok((output, rendered))
}

fn find_inline_scalar_span(line: &str) -> Option<(usize, usize)> {
    let mut in_code = false;
    let mut open = None;

    for (index, character) in line.char_indices() {
        if character == '`' {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if let Some(open) = open {
            if character == '}' {
                return Some((open, index));
            }
            continue;
        }
        if character == '{' && !line[index + 1..].starts_with('|') {
            open = Some(index);
        }
    }

    None
}

fn execute_block(
    executor: &mut impl SqlExecutor,
    sql: &str,
    kind: SqlBlockKind,
    pretty: bool,
) -> Result<String> {
    match kind {
        SqlBlockKind::Scalar => Ok(format!("{}\n", execute_scalar(executor, sql)?)),
        SqlBlockKind::Table => execute_table(executor, sql, pretty),
    }
}

fn execute_scalar(executor: &mut impl SqlExecutor, sql: &str) -> Result<String> {
    if sql.is_empty() {
        bail!("SQL block cannot be empty");
    }

    match executor.execute_sql(sql)? {
        QueryOutput::Statement { .. } => {
            bail!("plaintext SQL blocks must return exactly 1 column and 1 row: {sql}")
        }
        QueryOutput::Rows { headings, rows } => {
            if headings.len() != 1 {
                bail!(
                    "plaintext SQL blocks must return exactly 1 column; returned {} columns: {sql}",
                    headings.len()
                );
            }
            match rows.as_slice() {
                [] => {
                    bail!("plaintext SQL blocks must return exactly 1 row; returned 0 rows: {sql}")
                }
                [row] => Ok(row[0].clone()),
                _ => bail!(
                    "plaintext SQL blocks must return exactly 1 row; returned more than 1 row: {sql}"
                ),
            }
        }
    }
}

fn source_excerpt(source: &str, line_index: usize) -> String {
    const CONTEXT_LINES: usize = 2;

    let lines = source.lines().collect::<Vec<_>>();
    let start = line_index.saturating_sub(CONTEXT_LINES);
    let end = (line_index + CONTEXT_LINES + 1).min(lines.len());
    let line_number_width = end.to_string().len();

    lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, line)| {
            let current_index = start + offset;
            let marker = if current_index == line_index {
                ">"
            } else {
                " "
            };
            format!(
                "{marker} {:>line_number_width$} | {line}",
                current_index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn execute_table(executor: &mut impl SqlExecutor, sql: &str, pretty: bool) -> Result<String> {
    if sql.is_empty() {
        bail!("SQL block cannot be empty");
    }

    match executor.execute_sql(sql)? {
        QueryOutput::Statement { changed } => {
            Ok(format!("_Statement executed; {changed} row(s) changed._\n"))
        }
        QueryOutput::Rows { headings, rows } => {
            let headings = headings
                .iter()
                .map(|heading| escape_cell(heading))
                .collect::<Vec<_>>();
            let rows = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|value| escape_cell(value))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            Ok(render_table(&headings, &rows, pretty))
        }
    }
}

fn render_table(headings: &[String], rows: &[Vec<String>], pretty: bool) -> String {
    let widths = headings
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            rows.iter()
                .map(|row| row[index].chars().count())
                .chain([heading.chars().count(), 3])
                .max()
                .unwrap()
        })
        .collect::<Vec<_>>();

    let format_row = |cells: &[String]| {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                if pretty {
                    format!("{cell:<width$}", width = widths[index])
                } else {
                    cell.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };

    let mut table = String::new();
    table.push_str("| ");
    table.push_str(&format_row(headings));
    table.push_str(" |\n| ");
    table.push_str(
        &widths
            .iter()
            .map(|width| {
                if pretty {
                    "-".repeat(*width)
                } else {
                    "---".to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(" | "),
    );
    table.push_str(" |\n");

    for row in rows {
        table.push_str("| ");
        table.push_str(&format_row(row));
        table.push_str(" |\n");
    }

    table
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_owned(),
        Value::Integer(value) => value.to_string(),
        Value::Real(value) => value.to_string(),
        Value::Text(value) => value.to_owned(),
        Value::Blob(value) => format!("<{} byte blob>", value.len()),
    }
}

impl SqlExecutor for Connection {
    fn execute_sql(&mut self, sql: &str) -> Result<QueryOutput> {
        execute_sqlite(self, sql)
    }
}

impl SqlExecutor for &Connection {
    fn execute_sql(&mut self, sql: &str) -> Result<QueryOutput> {
        execute_sqlite(self, sql)
    }
}

impl SqlExecutor for postgres::Client {
    fn execute_sql(&mut self, sql: &str) -> Result<QueryOutput> {
        let messages = self
            .simple_query(sql)
            .with_context(|| format!("failed to execute SQL: {sql}"))?;
        let mut headings: Option<Vec<String>> = None;
        let mut rows = Vec::new();
        let mut changed = 0;

        for message in messages {
            match message {
                postgres::SimpleQueryMessage::Row(row) => {
                    if headings.is_none() {
                        headings = Some(
                            row.columns()
                                .iter()
                                .map(|column| column.name().to_owned())
                                .collect(),
                        );
                    }
                    rows.push(
                        (0..row.len())
                            .map(|index| row.get(index).unwrap_or("NULL").to_owned())
                            .collect(),
                    );
                }
                postgres::SimpleQueryMessage::CommandComplete(count) => {
                    changed = count;
                }
                _ => {}
            }
        }

        match headings {
            Some(headings) => Ok(QueryOutput::Rows { headings, rows }),
            None => Ok(QueryOutput::Statement { changed }),
        }
    }
}

impl SqlExecutor for mysql::Conn {
    fn execute_sql(&mut self, sql: &str) -> Result<QueryOutput> {
        let mut result = self
            .query_iter(sql)
            .with_context(|| format!("failed to execute SQL: {sql}"))?;
        let columns = result.columns();
        let columns = columns.as_ref();
        if columns.is_empty() {
            let changed = result.affected_rows();
            return Ok(QueryOutput::Statement { changed });
        }
        let headings = columns
            .iter()
            .map(|column| column.name_str().into_owned())
            .collect::<Vec<_>>();
        let mut rows = Vec::new();

        for row in result.by_ref() {
            let values = row
                .with_context(|| format!("failed to read result from SQL: {sql}"))?
                .unwrap();
            rows.push(values.iter().map(format_mysql_value).collect());
        }

        Ok(QueryOutput::Rows { headings, rows })
    }
}

fn execute_sqlite(connection: &Connection, sql: &str) -> Result<QueryOutput> {
    let mut statement = connection
        .prepare(sql)
        .with_context(|| format!("invalid SQL: {sql}"))?;

    if statement.column_count() == 0 {
        let changed = statement
            .execute([])
            .with_context(|| format!("failed to execute SQL: {sql}"))?;
        return Ok(QueryOutput::Statement {
            changed: changed as u64,
        });
    }

    let headings: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let column_count = headings.len();
    let rows = statement
        .query_map([], |row| {
            (0..column_count)
                .map(|index| row.get::<_, Value>(index))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .with_context(|| format!("failed to execute SQL: {sql}"))?;

    let rows = rows
        .map(|row| {
            row.with_context(|| format!("failed to read result from SQL: {sql}"))
                .map(|values| values.iter().map(format_value).collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(QueryOutput::Rows { headings, rows })
}

fn format_mysql_value(value: &mysql::Value) -> String {
    match value {
        mysql::Value::NULL => "NULL".to_owned(),
        mysql::Value::Bytes(value) => String::from_utf8_lossy(value).into_owned(),
        mysql::Value::Int(value) => value.to_string(),
        mysql::Value::UInt(value) => value.to_string(),
        mysql::Value::Float(value) => value.to_string(),
        mysql::Value::Double(value) => value.to_string(),
        mysql::Value::Date(year, month, day, hour, minute, second, micros) => {
            if *hour == 0 && *minute == 0 && *second == 0 && *micros == 0 {
                format!("{year:04}-{month:02}-{day:02}")
            } else if *micros == 0 {
                format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
            } else {
                format!(
                    "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{micros:06}"
                )
            }
        }
        mysql::Value::Time(negative, days, hours, minutes, seconds, micros) => {
            let sign = if *negative { "-" } else { "" };
            let hours = u32::from(*hours) + days * 24;
            if *micros == 0 {
                format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
            } else {
                format!("{sign}{hours:02}:{minutes:02}:{seconds:02}.{micros:06}")
            }
        }
    }
}

fn escape_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_query_as_a_markdown_table() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "# Users\n\n{|select 1 as id, 'Ada' as name|}\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(
            output,
            "# Users\n\n| id | name |\n| --- | --- |\n| 1 | Ada |\n"
        );
    }

    #[test]
    fn statements_can_prepare_data_for_later_queries() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "{|create table users (name text)|}\n{|insert into users values ('Ada')|}\n{|select name from users|}\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert!(output.contains("0 row(s) changed"));
        assert!(output.contains("1 row(s) changed"));
        assert!(output.ends_with("| name |\n| --- |\n| Ada |\n"));
    }

    #[test]
    fn reports_an_unclosed_block() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "# Report\n\nSome context\n{select 1\nfrom example\n";

        let error = render_with_connection(input, &connection, false).unwrap_err();

        assert_eq!(
            error.to_string(),
            "unclosed SQL block starting at line 4\n\n  2 | \n  3 | Some context\n> 4 | {select 1\n  5 | from example"
        );
    }

    #[test]
    fn accepts_table_blocks_in_any_sql_block_line_position() {
        let connection = Connection::open_in_memory().unwrap();
        let inputs = [
            "{|select 1 as value|}",
            "{|\nselect 1 as value\n|}",
            "{|select 1 as value\n|}",
            "{|\nselect 1 as value|}",
            "  {|  select 1 as value  |}  ",
        ];

        for input in inputs {
            let output = render_with_connection(input, &connection, false).unwrap();
            assert_eq!(output, "| value |\n| --- |\n| 1 |\n", "input: {input:?}");
        }
    }

    #[test]
    fn pretty_prints_markdown_table_columns() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "{|select 1 as id, 'Ada' as name union all select 200, 'Grace'|}\n";

        let output = render_with_connection(input, &connection, true).unwrap();

        assert_eq!(
            output,
            "| id  | name  |\n| --- | ----- |\n| 1   | Ada   |\n| 200 | Grace |\n"
        );
    }

    #[test]
    fn inlines_single_column_single_row_queries() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Hello {SELECT 'Ada'}!\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "Hello Ada!\n");
    }

    #[test]
    fn renders_standalone_scalar_blocks_as_plaintext() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "{SELECT 'Ada'}\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "Ada\n");
    }

    #[test]
    fn supports_multiple_inline_scalar_queries() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "{SELECT 'A'} + {SELECT 2;}\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "A + 2\n");
    }

    #[test]
    fn errors_when_plaintext_queries_return_multiple_columns() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Hello {SELECT 'Ada', 'Grace'}!\n";

        let error = render_with_connection(input, &connection, false).unwrap_err();

        assert_eq!(
            error.to_string(),
            "plaintext SQL blocks must return exactly 1 column; returned 2 columns: SELECT 'Ada', 'Grace'"
        );
    }

    #[test]
    fn errors_when_plaintext_queries_return_multiple_rows() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Hello {SELECT 'Ada' UNION ALL SELECT 'Grace'}!\n";

        let error = render_with_connection(input, &connection, false).unwrap_err();

        assert_eq!(
            error.to_string(),
            "plaintext SQL blocks must return exactly 1 row; returned more than 1 row: SELECT 'Ada' UNION ALL SELECT 'Grace'"
        );
    }

    #[test]
    fn errors_when_plaintext_queries_return_no_rows() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Hello {SELECT 'Ada' WHERE false}!\n";

        let error = render_with_connection(input, &connection, false).unwrap_err();

        assert_eq!(
            error.to_string(),
            "plaintext SQL blocks must return exactly 1 row; returned 0 rows: SELECT 'Ada' WHERE false"
        );
    }

    #[test]
    fn does_not_execute_inline_queries_in_fenced_code() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "```sql\nHello {SELECT 'Ada'}!\n```\n\nHello {SELECT 'Ada'}!\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "```sql\nHello {SELECT 'Ada'}!\n```\n\nHello Ada!\n");
    }

    #[test]
    fn does_not_execute_inline_queries_in_inline_code() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Use `{NAME}` before {SELECT 'Ada'}.\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "Use `{NAME}` before Ada.\n");
    }

    #[test]
    fn table_blocks_render_multi_column_results_as_tables() {
        let connection = Connection::open_in_memory().unwrap();
        let input = "Result:\n{|SELECT 1 AS a, 2 AS b|}\n";

        let output = render_with_connection(input, &connection, false).unwrap();

        assert_eq!(output, "Result:\n| a | b |\n| --- | --- |\n| 1 | 2 |\n");
    }
}
