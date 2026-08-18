use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, ValueEnum};
use mysql::Opts;
use mysql::prelude::Queryable;
use postgres::NoTls;
use rusqlite::Connection;

#[derive(Clone, Copy, Debug)]
enum OutputView {
    Markdown,
    Html,
}

#[derive(Clone, Copy, Debug)]
enum OutputArchive {
    Zip,
    TarGz,
}

#[derive(Clone, Debug)]
struct OutputTarget {
    view: OutputView,
    archive: Option<OutputArchive>,
}

struct RenderedPage {
    path: PathBuf,
    bytes: Vec<u8>,
}

enum Database {
    Sqlite(Connection),
    Postgres(postgres::Client),
    Mysql(mysql::Conn),
}

impl Database {
    fn execute_sql_file(&mut self, path: &Path) -> Result<()> {
        let sql = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read init SQL file {}", path.display()))?;

        match self {
            Database::Sqlite(connection) => connection
                .execute_batch(&sql)
                .with_context(|| format!("failed to execute init SQL from {}", path.display()))?,
            Database::Postgres(connection) => connection
                .batch_execute(&sql)
                .with_context(|| format!("failed to execute init SQL from {}", path.display()))?,
            Database::Mysql(connection) => connection
                .query_drop(&sql)
                .with_context(|| format!("failed to execute init SQL from {}", path.display()))?,
        }

        Ok(())
    }
}

impl squidown::SqlExecutor for Database {
    fn execute_sql(&mut self, sql: &str) -> Result<squidown::QueryOutput> {
        match self {
            Database::Sqlite(connection) => connection.execute_sql(sql),
            Database::Postgres(connection) => connection.execute_sql(sql),
            Database::Mysql(connection) => connection.execute_sql(sql),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum DatabaseDriver {
    Sqlite,
    Postgres,
    Mysql,
}

#[derive(Debug, Parser)]
#[command(version, about, disable_help_flag = true)]
struct Cli {
    /// SQUID files containing SQL blocks delimited by { and }.
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Database name, local SQLite path, or full postgresql://, postgres://, or mysql:// URI.
    #[arg(short = 'd', long = "dbname", visible_alias = "database")]
    dbname: Option<String>,

    /// Execute this SQL file against the database before rendering.
    #[arg(short = 'i', long)]
    init: Option<PathBuf>,

    /// Database host for Postgres/MySQL connections.
    #[arg(short = 'h', long)]
    host: Option<String>,

    /// Database port for Postgres/MySQL connections.
    #[arg(short = 'p', long)]
    port: Option<u16>,

    /// Database username for Postgres/MySQL connections.
    #[arg(short = 'U', long = "username")]
    username: Option<String>,

    /// Database password for Postgres/MySQL connections.
    #[arg(long)]
    password: Option<String>,

    /// Force the database driver when flags are ambiguous.
    #[arg(long, value_enum)]
    driver: Option<DatabaseDriver>,

    /// Write output path; use -, -.md, -.html for stdout or .zip, .tar.gz for archives.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Do not align generated Markdown table columns.
    #[arg(long)]
    compact: bool,

    /// Print help.
    #[arg(long, action = ArgAction::HelpLong)]
    help: Option<bool>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output_path = cli.output.unwrap_or_else(default_output_path);
    let target = output_target(&output_path)?;
    let (inputs, connection_target) = split_inputs_and_connection(cli.input, cli.dbname)?;

    if inputs.len() > 1 && target.archive.is_none() {
        bail!("multiple input files require --output with a .zip or .tar.gz extension");
    }
    if target.archive.is_some() && is_stdout_output(&output_path) {
        bail!("archive output cannot be written to stdout");
    }

    let mut database = open_database(
        connection_target,
        cli.driver,
        cli.host.as_deref(),
        cli.port,
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;

    if let Some(init) = cli.init.as_deref() {
        database.execute_sql_file(init)?;
    }

    let pages = render_pages(
        &inputs,
        &mut database,
        cli.compact,
        target.view,
        target.archive.is_some(),
    )?;

    if is_stdout_output(&output_path) {
        let page = pages
            .into_iter()
            .next()
            .expect("at least one input is required");
        print!("{}", String::from_utf8_lossy(&page.bytes));
    } else {
        match target.archive {
            Some(OutputArchive::Zip) => write_zip_archive(&output_path, &pages)?,
            Some(OutputArchive::TarGz) => write_tar_gz_archive(&output_path, &pages)?,
            None => {
                let page = pages
                    .into_iter()
                    .next()
                    .expect("at least one input is required");
                std::fs::write(&output_path, page.bytes)
                    .with_context(|| format!("failed to write {}", output_path.display()))?;
            }
        }
    }
    Ok(())
}

fn default_output_path() -> PathBuf {
    PathBuf::from("-")
}

fn output_target(output: &Path) -> Result<OutputTarget> {
    if output == Path::new("-") {
        return Ok(OutputTarget {
            view: OutputView::Markdown,
            archive: None,
        });
    }

    let Some(filename) = output.file_name().and_then(|filename| filename.to_str()) else {
        bail!(
            "output path {} has no file name; use .md, .html, .zip, or .tar.gz",
            output.display()
        );
    };
    let filename = filename.to_ascii_lowercase();

    if filename.ends_with(".html.tar.gz") || filename.ends_with(".htm.tar.gz") {
        return Ok(OutputTarget {
            view: OutputView::Html,
            archive: Some(OutputArchive::TarGz),
        });
    }
    if filename.ends_with(".md.tar.gz")
        || filename.ends_with(".markdown.tar.gz")
        || filename.ends_with(".tar.gz")
    {
        return Ok(OutputTarget {
            view: OutputView::Markdown,
            archive: Some(OutputArchive::TarGz),
        });
    }
    if filename.ends_with(".html.zip") || filename.ends_with(".htm.zip") {
        return Ok(OutputTarget {
            view: OutputView::Html,
            archive: Some(OutputArchive::Zip),
        });
    }
    if filename.ends_with(".md.zip")
        || filename.ends_with(".markdown.zip")
        || filename.ends_with(".zip")
    {
        return Ok(OutputTarget {
            view: OutputView::Markdown,
            archive: Some(OutputArchive::Zip),
        });
    }

    let Some(extension) = output.extension().and_then(|extension| extension.to_str()) else {
        bail!(
            "output path {} has no file extension; use .md, .html, .zip, or .tar.gz",
            output.display()
        );
    };
    match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => Ok(OutputTarget {
            view: OutputView::Markdown,
            archive: None,
        }),
        "html" | "htm" => Ok(OutputTarget {
            view: OutputView::Html,
            archive: None,
        }),
        _ => bail!("unsupported output file extension .{extension}"),
    }
}

fn is_stdout_output(output: &Path) -> bool {
    output == Path::new("-")
        || output.components().count() == 1
            && output.file_stem().and_then(|stem| stem.to_str()) == Some("-")
}

fn split_inputs_and_connection(
    mut inputs: Vec<PathBuf>,
    dbname: Option<String>,
) -> Result<(Vec<PathBuf>, Option<String>)> {
    if dbname.is_some() {
        return Ok((inputs, dbname));
    }

    if inputs.len() > 1
        && inputs
            .last()
            .and_then(|input| input.to_str())
            .is_some_and(is_connection_uri)
    {
        let connection = inputs
            .pop()
            .and_then(|input| input.into_os_string().into_string().ok())
            .expect("connection URI was valid UTF-8");
        return Ok((inputs, Some(connection)));
    }

    Ok((inputs, None))
}

fn open_database(
    dbname: Option<String>,
    driver: Option<DatabaseDriver>,
    host: Option<&str>,
    port: Option<u16>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Database> {
    let dbname = resolve_dbname(dbname);
    let inferred_driver = infer_driver(&dbname, driver, host, port, username, password)?;

    match inferred_driver {
        DatabaseDriver::Sqlite => {
            if host.is_some() || port.is_some() || username.is_some() || password.is_some() {
                bail!(
                    "SQLite connections do not support --host, --port, --username, or --password"
                );
            }
            let connection = Connection::open(&dbname)
                .with_context(|| format!("failed to open SQLite database {dbname}"))?;
            Ok(Database::Sqlite(connection))
        }
        DatabaseDriver::Postgres => {
            let connection_string = if is_connection_uri(&dbname) {
                dbname
            } else {
                build_postgres_connection_string(&dbname, host, port, username, password)
            };
            let connection = postgres::Client::connect(&connection_string, NoTls)
                .with_context(|| "failed to connect to PostgreSQL")?;
            Ok(Database::Postgres(connection))
        }
        DatabaseDriver::Mysql => {
            let url = if is_connection_uri(&dbname) {
                dbname
            } else {
                build_mysql_url(&dbname, host, port, username, password)?
            };
            let opts = Opts::from_url(&url).with_context(|| "invalid MySQL connection URL")?;
            let connection =
                mysql::Conn::new(opts).with_context(|| "failed to connect to MySQL")?;
            Ok(Database::Mysql(connection))
        }
    }
}

fn resolve_dbname(dbname: Option<String>) -> String {
    dbname
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| ":memory:".to_owned())
}

fn infer_driver(
    dbname: &str,
    driver: Option<DatabaseDriver>,
    host: Option<&str>,
    port: Option<u16>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<DatabaseDriver> {
    if let Some(driver) = driver {
        return Ok(driver);
    }
    if dbname.starts_with("postgresql://") || dbname.starts_with("postgres://") {
        return Ok(DatabaseDriver::Postgres);
    }
    if dbname.starts_with("mysql://") {
        return Ok(DatabaseDriver::Mysql);
    }
    if dbname.contains("://") {
        bail!("unsupported database URI scheme in {dbname}");
    }
    if host.is_some() || port.is_some() || username.is_some() || password.is_some() {
        return Ok(DatabaseDriver::Postgres);
    }
    Ok(DatabaseDriver::Sqlite)
}

fn is_connection_uri(value: &str) -> bool {
    value.starts_with("postgresql://")
        || value.starts_with("postgres://")
        || value.starts_with("mysql://")
}

fn build_postgres_connection_string(
    dbname: &str,
    host: Option<&str>,
    port: Option<u16>,
    username: Option<&str>,
    password: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(host) = host {
        parts.push(format!("host={}", escape_postgres_param(host)));
    }
    if let Some(port) = port {
        parts.push(format!("port={port}"));
    }
    if let Some(username) = username {
        parts.push(format!("user={}", escape_postgres_param(username)));
    }
    if let Some(password) = password {
        parts.push(format!("password={}", escape_postgres_param(password)));
    }
    parts.push(format!("dbname={}", escape_postgres_param(dbname)));
    parts.join(" ")
}

fn escape_postgres_param(value: &str) -> String {
    if value.chars().any(char::is_whitespace) || value.contains(['\\', '\'']) {
        format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
    } else {
        value.to_owned()
    }
}

fn build_mysql_url(
    dbname: &str,
    host: Option<&str>,
    port: Option<u16>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let host = host.unwrap_or("localhost");
    let mut url = url::Url::parse("mysql://localhost").expect("valid base MySQL URL");
    url.set_host(Some(host))
        .with_context(|| format!("invalid MySQL host {host}"))?;
    if let Some(port) = port {
        url.set_port(Some(port))
            .map_err(|_| anyhow::anyhow!("invalid MySQL port {port}"))?;
    }
    if let Some(username) = username {
        url.set_username(username)
            .map_err(|_| anyhow::anyhow!("invalid MySQL username"))?;
    }
    if let Some(password) = password {
        url.set_password(Some(password))
            .map_err(|_| anyhow::anyhow!("invalid MySQL password"))?;
    }
    url.set_path(dbname);
    Ok(url.to_string())
}

fn render_pages(
    inputs: &[PathBuf],
    database: &mut impl squidown::SqlExecutor,
    compact: bool,
    view: OutputView,
    rewrite_references: bool,
) -> Result<Vec<RenderedPage>> {
    let entry_root = common_entry_root(inputs);
    inputs
        .iter()
        .map(|input| {
            let mut markdown = std::fs::read_to_string(input)
                .with_context(|| format!("failed to read {}", input.display()))?;
            if rewrite_references {
                markdown = rewrite_squid_references(&markdown, view);
            }
            let markdown = if compact {
                squidown::render_compact_with_executor(&markdown, database)
            } else {
                squidown::render_pretty_with_executor(&markdown, database)
            }
            .with_context(|| format!("failed to render {}", input.display()))?;
            let output = match view {
                OutputView::Markdown => markdown,
                OutputView::Html => {
                    let filename = input
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("README.md");
                    squidown::render_github_html_named(&markdown, filename)
                }
            };
            Ok(RenderedPage {
                path: archive_entry_path(input, &entry_root, view),
                bytes: output.into_bytes(),
            })
        })
        .collect()
}

fn rewrite_squid_references(markdown: &str, view: OutputView) -> String {
    markdown
        .lines()
        .map(|line| rewrite_squid_references_in_line(line, view))
        .collect::<Vec<_>>()
        .join("\n")
        + if markdown.ends_with('\n') { "\n" } else { "" }
}

fn rewrite_squid_references_in_line(line: &str, view: OutputView) -> String {
    if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
        return line.to_owned();
    }

    let mut output = String::new();
    let mut remainder = line;
    while let Some(start) = remainder.find("](") {
        output.push_str(&remainder[..start + 2]);
        let after_start = &remainder[start + 2..];
        let Some(end) = after_start.find(')') else {
            output.push_str(after_start);
            return output;
        };
        let target = &after_start[..end];
        output.push_str(&rewrite_squid_reference_target(target, view));
        output.push(')');
        remainder = &after_start[end + 1..];
    }
    output.push_str(remainder);
    output
}

fn rewrite_squid_reference_target(target: &str, view: OutputView) -> String {
    if target.contains("://") || target.starts_with("mailto:") || target.starts_with('#') {
        return target.to_owned();
    }

    let split = target.find(['#', '?']).unwrap_or(target.len());
    let (path, suffix) = target.split_at(split);
    if !path.to_ascii_lowercase().ends_with(".squid") {
        return target.to_owned();
    }

    let mut output = String::with_capacity(target.len());
    output.push_str(&path[..path.len() - ".squid".len()]);
    output.push_str(match view {
        OutputView::Markdown => ".md",
        OutputView::Html => ".html",
    });
    output.push_str(suffix);
    output
}

fn common_entry_root(inputs: &[PathBuf]) -> PathBuf {
    let mut parents = inputs.iter().map(|input| {
        input
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let Some(first) = parents.next() else {
        return PathBuf::new();
    };
    parents.fold(first, common_path_prefix)
}

fn common_path_prefix(left: PathBuf, right: PathBuf) -> PathBuf {
    let mut prefix = PathBuf::new();
    for (left, right) in left.components().zip(right.components()) {
        if left != right {
            break;
        }
        prefix.push(left.as_os_str());
    }
    prefix
}

fn archive_entry_path(input: &Path, root: &Path, view: OutputView) -> PathBuf {
    let relative = input.strip_prefix(root).unwrap_or(input);
    let mut entry = relative.to_path_buf();
    entry.set_extension(match view {
        OutputView::Markdown => "md",
        OutputView::Html => "html",
    });
    normalize_archive_path(&entry)
}

fn normalize_archive_path(path: &Path) -> PathBuf {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(PathBuf::from(part)),
            _ => None,
        })
        .fold(PathBuf::new(), |mut output, part| {
            output.push(part);
            output
        })
}

fn write_zip_archive(path: &Path, pages: &[RenderedPage]) -> Result<()> {
    let mut output = Vec::new();
    let mut central_directory = Vec::new();

    for page in pages {
        let name = archive_path_string(&page.path)?;
        let name_bytes = name.as_bytes();
        let crc = crc32(&page.bytes);
        let local_header_offset = output.len() as u32;
        write_u32(&mut output, 0x0403_4b50)?;
        write_u16(&mut output, 20)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 0)?;
        write_u16(&mut output, 33)?;
        write_u32(&mut output, crc)?;
        write_u32(&mut output, page.bytes.len() as u32)?;
        write_u32(&mut output, page.bytes.len() as u32)?;
        write_u16(&mut output, name_bytes.len() as u16)?;
        write_u16(&mut output, 0)?;
        output.write_all(name_bytes)?;
        output.write_all(&page.bytes)?;

        write_u32(&mut central_directory, 0x0201_4b50)?;
        write_u16(&mut central_directory, 20)?;
        write_u16(&mut central_directory, 20)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 33)?;
        write_u32(&mut central_directory, crc)?;
        write_u32(&mut central_directory, page.bytes.len() as u32)?;
        write_u32(&mut central_directory, page.bytes.len() as u32)?;
        write_u16(&mut central_directory, name_bytes.len() as u16)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 0)?;
        write_u16(&mut central_directory, 0)?;
        write_u32(&mut central_directory, 0)?;
        write_u32(&mut central_directory, local_header_offset)?;
        central_directory.write_all(name_bytes)?;
    }

    let central_directory_offset = output.len() as u32;
    let central_directory_size = central_directory.len() as u32;
    output.write_all(&central_directory)?;
    write_u32(&mut output, 0x0605_4b50)?;
    write_u16(&mut output, 0)?;
    write_u16(&mut output, 0)?;
    write_u16(&mut output, pages.len() as u16)?;
    write_u16(&mut output, pages.len() as u16)?;
    write_u32(&mut output, central_directory_size)?;
    write_u32(&mut output, central_directory_offset)?;
    write_u16(&mut output, 0)?;

    std::fs::write(path, output).with_context(|| format!("failed to write {}", path.display()))
}

fn write_tar_gz_archive(path: &Path, pages: &[RenderedPage]) -> Result<()> {
    let mut tar = Vec::new();
    for page in pages {
        write_tar_entry(&mut tar, page)?;
    }
    tar.extend([0; 1024]);
    let gzip = gzip_stored(&tar)?;
    std::fs::write(path, gzip).with_context(|| format!("failed to write {}", path.display()))
}

fn write_tar_entry(output: &mut Vec<u8>, page: &RenderedPage) -> Result<()> {
    let name = archive_path_string(&page.path)?;
    if name.len() > 100 {
        bail!("archive entry path is too long for tar: {name}");
    }

    let mut header = [0u8; 512];
    write_tar_str(&mut header[0..100], &name);
    write_tar_octal(&mut header[100..108], 0o644);
    write_tar_octal(&mut header[108..116], 0);
    write_tar_octal(&mut header[116..124], 0);
    write_tar_octal(&mut header[124..136], page.bytes.len() as u64);
    write_tar_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'0';
    write_tar_str(&mut header[257..263], "ustar\0");
    write_tar_str(&mut header[263..265], "00");
    let checksum = header.iter().map(|byte| u32::from(*byte)).sum::<u32>();
    write_tar_checksum(&mut header[148..156], checksum);

    output.write_all(&header)?;
    output.write_all(&page.bytes)?;
    let padding = (512 - page.bytes.len() % 512) % 512;
    output.extend(std::iter::repeat_n(0, padding));
    Ok(())
}

fn gzip_stored(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255];
    for (index, chunk) in data.chunks(65_535).enumerate() {
        let final_block = index == data.len().saturating_sub(1) / 65_535;
        output.push(if final_block { 1 } else { 0 });
        write_u16(&mut output, chunk.len() as u16)?;
        write_u16(&mut output, !(chunk.len() as u16))?;
        output.write_all(chunk)?;
    }
    write_u32(&mut output, crc32(data))?;
    write_u32(&mut output, data.len() as u32)?;
    Ok(output)
}

fn archive_path_string(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .with_context(|| format!("archive entry path is not valid UTF-8: {}", path.display()))?
        .replace('\\', "/");
    if value.is_empty() || value.starts_with('/') || value.contains("../") || value == ".." {
        bail!("invalid archive entry path: {value}");
    }
    Ok(value)
}

fn write_tar_str(field: &mut [u8], value: &str) {
    let bytes = value.as_bytes();
    let len = bytes.len().min(field.len());
    field[..len].copy_from_slice(&bytes[..len]);
}

fn write_tar_octal(field: &mut [u8], value: u64) {
    let formatted = format!("{value:0width$o}\0", width = field.len() - 1);
    field.copy_from_slice(formatted.as_bytes());
}

fn write_tar_checksum(field: &mut [u8], value: u32) {
    let formatted = format!("{value:06o}\0 ",);
    field.copy_from_slice(formatted.as_bytes());
}

fn write_u16(output: &mut Vec<u8>, value: u16) -> Result<()> {
    output.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u32(output: &mut Vec<u8>, value: u32) -> Result<()> {
    output.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_output_is_the_default() {
        let cli = Cli::try_parse_from(["squid", "report.squid"]).unwrap();

        assert!(!cli.compact);
    }

    #[test]
    fn compact_is_a_valueless_flag() {
        let cli = Cli::try_parse_from(["squid", "report.squid", "--compact"]).unwrap();

        assert!(cli.compact);
    }

    #[test]
    fn output_accepts_a_file_path() {
        let cli =
            Cli::try_parse_from(["squid", "report.squid", "--output", "rendered.md"]).unwrap();

        assert_eq!(cli.output, Some(PathBuf::from("rendered.md")));
    }

    #[test]
    fn dbname_accepts_a_sqlite_path() {
        let cli = Cli::try_parse_from(["squid", "report.squid", "-d", "benchmarks.db"]).unwrap();

        assert_eq!(cli.dbname, Some("benchmarks.db".to_owned()));
    }

    #[test]
    fn database_remains_an_alias_for_dbname() {
        let cli =
            Cli::try_parse_from(["squid", "report.squid", "--database", "benchmarks.db"]).unwrap();

        assert_eq!(cli.dbname, Some("benchmarks.db".to_owned()));
    }

    #[test]
    fn init_accepts_a_file_path() {
        let cli = Cli::try_parse_from(["squid", "report.squid", "--init", "setup.sql"]).unwrap();

        assert_eq!(cli.init, Some(PathBuf::from("setup.sql")));
    }

    #[test]
    fn init_short_flag_accepts_a_file_path() {
        let cli = Cli::try_parse_from(["squid", "report.squid", "-i", "setup.sql"]).unwrap();

        assert_eq!(cli.init, Some(PathBuf::from("setup.sql")));
    }

    #[test]
    fn database_url_is_used_when_dbname_is_omitted() {
        let previous = std::env::var("DATABASE_URL").ok();
        unsafe {
            std::env::set_var(
                "DATABASE_URL",
                "postgresql://alice:secret@localhost/benchmarks",
            );
        }

        assert_eq!(
            resolve_dbname(None),
            "postgresql://alice:secret@localhost/benchmarks"
        );
        assert_eq!(resolve_dbname(Some("local.db".to_owned())), "local.db");

        unsafe {
            match previous {
                Some(value) => std::env::set_var("DATABASE_URL", value),
                None => std::env::remove_var("DATABASE_URL"),
            }
        }
    }

    #[test]
    fn init_sql_runs_against_default_in_memory_sqlite_database() {
        let path = std::env::temp_dir().join(format!(
            "squid-init-{}-{}.sql",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        std::fs::write(
            &path,
            "create table users (name text); insert into users values ('Ada');",
        )
        .unwrap();

        let mut database = open_database(None, None, None, None, None, None).unwrap();
        database.execute_sql_file(&path).unwrap();
        let output =
            squidown::render_compact_with_executor("{|select name from users|}", &mut database)
                .unwrap();

        let _ = std::fs::remove_file(&path);

        assert_eq!(output, "| name |\n| --- |\n| Ada |\n");
    }

    #[test]
    fn psql_style_connection_flags_are_accepted() {
        let cli = Cli::try_parse_from([
            "squid",
            "report.squid",
            "-h",
            "localhost",
            "-p",
            "5432",
            "-U",
            "alice",
            "-d",
            "benchmarks",
        ])
        .unwrap();

        assert_eq!(cli.host, Some("localhost".to_owned()));
        assert_eq!(cli.port, Some(5432));
        assert_eq!(cli.username, Some("alice".to_owned()));
        assert_eq!(cli.dbname, Some("benchmarks".to_owned()));
    }

    #[test]
    fn positional_connection_uri_is_split_from_inputs() {
        let (inputs, dbname) = split_inputs_and_connection(
            vec![
                PathBuf::from("report.squid"),
                PathBuf::from("postgresql://alice:secret@localhost:5432/benchmarks"),
            ],
            None,
        )
        .unwrap();

        assert_eq!(inputs, vec![PathBuf::from("report.squid")]);
        assert_eq!(
            dbname,
            Some("postgresql://alice:secret@localhost:5432/benchmarks".to_owned())
        );
    }

    #[test]
    fn file_paths_infer_sqlite_unless_connection_flags_are_present() {
        assert_eq!(
            infer_driver("benchmarks.db", None, None, None, None, None).unwrap(),
            DatabaseDriver::Sqlite
        );
        assert_eq!(
            infer_driver("benchmarks", None, Some("localhost"), None, None, None).unwrap(),
            DatabaseDriver::Postgres
        );
    }

    #[test]
    fn accepts_multiple_input_paths() {
        let cli = Cli::try_parse_from(["squid", "index.squid", "users.squid"]).unwrap();

        assert_eq!(
            cli.input,
            vec![PathBuf::from("index.squid"), PathBuf::from("users.squid")]
        );
    }

    #[test]
    fn output_format_is_not_a_cli_flag() {
        let error =
            Cli::try_parse_from(["squid", "report.squid", "--format", "html-ghm"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn default_output_path_uses_stdout() {
        assert_eq!(default_output_path(), PathBuf::from("-"));
    }

    #[test]
    fn output_target_comes_from_file_extension() {
        assert!(matches!(
            output_target(Path::new("report.md")).unwrap(),
            OutputTarget {
                view: OutputView::Markdown,
                archive: None
            }
        ));
        assert!(matches!(
            output_target(Path::new("report.html")).unwrap(),
            OutputTarget {
                view: OutputView::Html,
                archive: None
            }
        ));
        assert!(matches!(
            output_target(Path::new("report.HTML")).unwrap(),
            OutputTarget {
                view: OutputView::Html,
                archive: None
            }
        ));
        assert!(matches!(
            output_target(Path::new("-")).unwrap(),
            OutputTarget {
                view: OutputView::Markdown,
                archive: None
            }
        ));
        assert!(matches!(
            output_target(Path::new("-.md")).unwrap(),
            OutputTarget {
                view: OutputView::Markdown,
                archive: None
            }
        ));
        assert!(matches!(
            output_target(Path::new("-.html")).unwrap(),
            OutputTarget {
                view: OutputView::Html,
                archive: None
            }
        ));
        assert!(output_target(Path::new("report.txt")).is_err());
    }

    #[test]
    fn output_target_supports_archives() {
        assert!(matches!(
            output_target(Path::new("content.zip")).unwrap(),
            OutputTarget {
                view: OutputView::Markdown,
                archive: Some(OutputArchive::Zip)
            }
        ));
        assert!(matches!(
            output_target(Path::new("content.html.zip")).unwrap(),
            OutputTarget {
                view: OutputView::Html,
                archive: Some(OutputArchive::Zip)
            }
        ));
        assert!(matches!(
            output_target(Path::new("content.tar.gz")).unwrap(),
            OutputTarget {
                view: OutputView::Markdown,
                archive: Some(OutputArchive::TarGz)
            }
        ));
        assert!(matches!(
            output_target(Path::new("content.html.tar.gz")).unwrap(),
            OutputTarget {
                view: OutputView::Html,
                archive: Some(OutputArchive::TarGz)
            }
        ));
    }

    #[test]
    fn archive_entry_paths_keep_structure_under_common_parent() {
        let inputs = vec![
            PathBuf::from("examples/multi-page/index.squid"),
            PathBuf::from("examples/multi-page/child/index.squid"),
        ];
        let root = common_entry_root(&inputs);

        assert_eq!(root, PathBuf::from("examples/multi-page"));
        assert_eq!(
            archive_entry_path(&inputs[0], &root, OutputView::Markdown),
            PathBuf::from("index.md")
        );
        assert_eq!(
            archive_entry_path(&inputs[1], &root, OutputView::Html),
            PathBuf::from("child/index.html")
        );
    }

    #[test]
    fn archive_references_rewrite_squid_links_to_output_extension() {
        let input = "- [users](./users.squid)\n- [child](child/index.squid#top)\n- [external](https://example.com/x.squid)\n";

        assert_eq!(
            rewrite_squid_references(input, OutputView::Markdown),
            "- [users](./users.md)\n- [child](child/index.md#top)\n- [external](https://example.com/x.squid)\n"
        );
        assert_eq!(
            rewrite_squid_references(input, OutputView::Html),
            "- [users](./users.html)\n- [child](child/index.html#top)\n- [external](https://example.com/x.squid)\n"
        );
    }

    #[test]
    fn stdout_output_supports_extension_variants() {
        assert!(is_stdout_output(Path::new("-")));
        assert!(is_stdout_output(Path::new("-.md")));
        assert!(is_stdout_output(Path::new("-.html")));
        assert!(!is_stdout_output(Path::new("report.squid")));
        assert!(!is_stdout_output(Path::new("reports/-.html")));
    }
}
