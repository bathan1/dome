use std::time::{SystemTime, UNIX_EPOCH};

use pulldown_cmark::{Options, Parser, html};

const PAGE_START: &str = r#"<!doctype html>
<html lang="en" data-theme="github-system" data-color-mode="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>SQUID report</title>
<script>
(() => {
  const system = window.matchMedia("(prefers-color-scheme: dark)");
  document.documentElement.dataset.colorMode = system.matches ? "dark" : "light";
})();
</script>
<style>
:root,
[data-color-mode="light"] {
  color-scheme: light;
  --bgColor-default: #ffffff;
  --bgColor-muted: #f6f8fa;
  --bgColor-neutral-muted: rgba(175, 184, 193, .2);
  --borderColor-default: #d1d9e0;
  --borderColor-muted: #d8dee4;
  --fgColor-default: #1f2328;
  --fgColor-muted: #59636e;
  --fgColor-accent: #0969da;
  --button-bg: #f6f8fa;
  --button-hover-bg: #eff2f5;
}
[data-color-mode="dark"] {
  color-scheme: dark;
  --bgColor-default: #0d1117;
  --bgColor-muted: #151b23;
  --bgColor-neutral-muted: rgba(110, 118, 129, .4);
  --borderColor-default: #3d444d;
  --borderColor-muted: #3d444d;
  --fgColor-default: #f0f6fc;
  --fgColor-muted: #9198a1;
  --fgColor-accent: #4493f8;
  --button-bg: #212830;
  --button-hover-bg: #262c36;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  color: var(--fgColor-default);
  background: var(--bgColor-default);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans",
    Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
  font-size: 14px;
  line-height: 1.5;
  word-wrap: break-word;
}
.page {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 296px;
  gap: 24px;
  width: 100%;
  max-width: 1280px;
  margin: 0 auto;
  padding: 24px;
}
.document { min-width: 0; }
.file-header {
  position: sticky;
  z-index: 10;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 48px;
  padding: 8px 12px;
  background: var(--bgColor-default);
  border: 1px solid var(--borderColor-default);
  border-bottom: 0;
  border-radius: 6px 6px 0 0;
}
.file-name {
  display: flex;
  gap: 8px;
  align-items: center;
  min-width: 0;
  font-size: 14px;
  font-weight: 600;
}
.file-name span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.theme-control {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.theme-control span {
  color: var(--fgColor-muted);
  font-size: 12px;
  font-weight: 500;
}
.theme-select {
  min-width: 144px;
  height: 32px;
  padding: 0 28px 0 10px;
  color: var(--fgColor-default);
  background: var(--button-bg);
  border: 1px solid var(--borderColor-default);
  border-radius: 6px;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
}
.theme-select:hover { background: var(--button-hover-bg); }
.theme-select:focus-visible {
  outline: 2px solid var(--fgColor-accent);
  outline-offset: 2px;
}
.readme {
  min-width: 0;
  padding: 32px;
  border: 1px solid var(--borderColor-default);
  border-radius: 0 0 6px 6px;
}
.about {
  min-width: 0;
  padding-top: 8px;
}
.about h2 {
  margin: 0 0 16px;
  font-size: 16px;
  font-weight: 600;
}
.about p {
  margin: 0;
  color: var(--fgColor-muted);
  font-size: 12px;
}
.about time { white-space: nowrap; }
.markdown-body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans",
    Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
  font-size: 16px;
  line-height: 1.5;
}
.markdown-body::before,
.markdown-body::after { display: table; content: ""; }
.markdown-body::after { clear: both; }
.markdown-body > :first-child { margin-top: 0 !important; }
.markdown-body > :last-child { margin-bottom: 0 !important; }
.markdown-body a {
  color: var(--fgColor-accent);
  text-decoration: none;
}
.markdown-body a:hover { text-decoration: underline; }
.markdown-body p,
.markdown-body blockquote,
.markdown-body ul,
.markdown-body ol,
.markdown-body dl,
.markdown-body table,
.markdown-body pre,
.markdown-body details {
  margin-top: 0;
  margin-bottom: 16px;
}
.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
  margin-top: 24px;
  margin-bottom: 16px;
  font-weight: 600;
  line-height: 1.25;
}
.markdown-body h1 {
  padding-bottom: .3em;
  font-size: 2em;
  border-bottom: 1px solid var(--borderColor-muted);
}
.markdown-body h2 {
  padding-bottom: .3em;
  font-size: 1.5em;
  border-bottom: 1px solid var(--borderColor-muted);
}
.markdown-body h3 { font-size: 1.25em; }
.markdown-body h4 { font-size: 1em; }
.markdown-body h5 { font-size: .875em; }
.markdown-body h6 { color: var(--fgColor-muted); font-size: .85em; }
.markdown-body hr {
  height: .25em;
  margin: 24px 0;
  padding: 0;
  overflow: hidden;
  background-color: var(--borderColor-default);
  border: 0;
}
.markdown-body blockquote {
  padding: 0 1em;
  color: var(--fgColor-muted);
  border-left: .25em solid var(--borderColor-default);
}
.markdown-body blockquote > :first-child { margin-top: 0; }
.markdown-body blockquote > :last-child { margin-bottom: 0; }
.markdown-body ul,
.markdown-body ol { padding-left: 2em; }
.markdown-body li + li { margin-top: .25em; }
.markdown-body code,
.markdown-body tt {
  margin: 0;
  padding: .2em .4em;
  font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas,
    Liberation Mono, monospace;
  font-size: 85%;
  white-space: break-spaces;
  background-color: var(--bgColor-neutral-muted);
  border-radius: 6px;
}
.markdown-body pre {
  padding: 16px;
  overflow: auto;
  font-size: 85%;
  line-height: 1.45;
  background-color: var(--bgColor-muted);
  border-radius: 6px;
}
.markdown-body pre code {
  display: inline;
  margin: 0;
  padding: 0;
  overflow: visible;
  line-height: inherit;
  word-wrap: normal;
  background-color: transparent;
  border: 0;
}
.markdown-body table {
  display: block;
  width: max-content;
  max-width: 100%;
  overflow: auto;
  border-spacing: 0;
  border-collapse: collapse;
}
.markdown-body table th {
  font-weight: 600;
}
.markdown-body table th,
.markdown-body table td {
  padding: 6px 13px;
  border: 1px solid var(--borderColor-default);
}
.markdown-body table tr {
  background-color: var(--bgColor-default);
  border-top: 1px solid var(--borderColor-default);
}
.markdown-body table tr:nth-child(2n) {
  background-color: var(--bgColor-muted);
}
.markdown-body img {
  max-width: 100%;
  box-sizing: content-box;
  background-color: var(--bgColor-default);
}
.markdown-body input[type="checkbox"] {
  margin: 0 .2em .25em -1.4em;
  vertical-align: middle;
}
@media (max-width: 767px) {
  .file-header {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }
  .theme-control {
    width: 100%;
  }
  .theme-select {
    flex: 1;
  }
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 16px;
  }
  .readme {
    padding: 24px;
  }
  .about {
    order: 2;
    padding: 0;
  }
}
</style>
</head>
<body>
<main class="page">
"#;

const CONTENT_START: &str = r#"</span>
</div>
<label class="theme-control" for="theme-select">
<span>Theme</span>
<select id="theme-select" class="theme-select">
<option value="github-system" selected>GitHub system</option>
<option value="github-light">GitHub light</option>
<option value="github-dark">GitHub dark</option>
</select>
</label>
</header>
<article class="readme markdown-body">
"#;

const SIDEBAR_START: &str = r#"</article>
</section>
<aside class="about" aria-labelledby="about-heading">
<h2 id="about-heading">About</h2>
<p>Generated at <time datetime=""#;

const PAGE_END: &str = r#"</time></p>
</aside>
</main>
<script>
(() => {
  const root = document.documentElement;
  const select = document.getElementById("theme-select");
  const system = window.matchMedia("(prefers-color-scheme: dark)");
  const resolveTheme = () => {
    root.dataset.theme = select.value;
    root.dataset.colorMode =
      select.value === "github-dark" ||
      (select.value === "github-system" && system.matches)
        ? "dark"
        : "light";
  };
  select.addEventListener("change", resolveTheme);
  system.addEventListener("change", resolveTheme);
  resolveTheme();

  const generatedAt = document.querySelector(".about time");
  const date = new Date(generatedAt.dateTime);
  const parts = new Intl.DateTimeFormat(undefined, {
    month: "2-digit", day: "2-digit", year: "numeric",
    hour: "2-digit", minute: "2-digit", hour12: true,
    timeZoneName: "short"
  }).formatToParts(date);
  const part = type => parts.find(item => item.type === type)?.value ?? "";
  generatedAt.textContent =
    `${part("month")}/${part("day")}/${part("year")} ` +
    `${part("hour")}:${part("minute")} ${part("dayPeriod")} ` +
    `(${part("timeZoneName")})`;
})();
</script>
</body>
</html>
"#;

/// Convert rendered Markdown into a self-contained GitHub-style HTML page.
pub fn render_github_html(markdown: &str) -> String {
    render_github_html_named(markdown, "README.md")
}

/// Convert rendered Markdown into GitHub-style HTML labeled with its source filename.
pub fn render_github_html_named(markdown: &str, filename: &str) -> String {
    let generated_at = SystemTime::now();
    render_github_html_at(
        markdown,
        filename,
        &utc_timestamp(generated_at),
        &utc_display_timestamp(generated_at),
    )
}

fn render_github_html_at(
    markdown: &str,
    filename: &str,
    generated_at: &str,
    generated_at_display: &str,
) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut rendered_markdown = String::new();
    html::push_html(&mut rendered_markdown, Parser::new_ext(markdown, options));

    let mut page = String::with_capacity(PAGE_START.len() + rendered_markdown.len() + 256);
    page.push_str(PAGE_START);
    page.push_str(
        r#"<section class="document">
<header class="file-header">
<div class="file-name">
<svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
<span>"#,
    );
    push_escaped_html(&mut page, filename);
    page.push_str(CONTENT_START);
    page.push_str(&rendered_markdown);
    page.push_str(SIDEBAR_START);
    page.push_str(generated_at);
    page.push_str(r#"">"#);
    page.push_str(generated_at_display);
    page.push_str(PAGE_END);
    page
}

fn push_escaped_html(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
}

fn utc_timestamp(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let seconds_in_day = seconds % 86_400;
    let (year, month, day) = civil_date_from_days(days);
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day % 3_600 / 60;
    let second = seconds_in_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn utc_display_timestamp(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let seconds_in_day = seconds % 86_400;
    let (year, month, day) = civil_date_from_days(days);
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day % 3_600 / 60;
    let (hour, period) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };

    format!("{month:02}/{day:02}/{year:04} {hour:02}:{minute:02} {period} (UTC)")
}

fn civil_date_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_github_flavored_markdown_and_metadata() {
        let output = render_github_html_at(
            "# Report\n\n| id | name |\n| --- | --- |\n| 1 | Ada |\n",
            "report.squid",
            "2026-07-03T12:34:56Z",
            "07/03/2026 12:34 PM (UTC)",
        );

        assert!(output.starts_with("<!doctype html>"));
        assert!(
            output
                .contains(r#"<html lang="en" data-theme="github-system" data-color-mode="light">"#)
        );
        assert!(output.contains("<span>report.squid</span>"));
        assert!(output.contains(r#"<select id="theme-select" class="theme-select">"#));
        assert!(
            output.contains(r#"<option value="github-system" selected>GitHub system</option>"#)
        );
        assert!(output.contains(r#"<option value="github-light">GitHub light</option>"#));
        assert!(output.contains(r#"<option value="github-dark">GitHub dark</option>"#));
        assert!(output.contains(r#"window.matchMedia("(prefers-color-scheme: dark)")"#));
        assert!(output.contains(r#"<article class="readme markdown-body">"#));
        assert!(output.contains("<table>"));
        assert!(output.contains("<td>Ada</td>"));
        assert!(output.contains("<h2 id=\"about-heading\">About</h2>"));
        assert!(
            output.contains(
                r#"<time datetime="2026-07-03T12:34:56Z">07/03/2026 12:34 PM (UTC)</time>"#
            )
        );
        assert!(output.contains("grid-template-columns: minmax(0, 1fr) 296px"));
    }

    #[test]
    fn formats_utc_timestamps() {
        assert_eq!(utc_timestamp(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(
            utc_timestamp(UNIX_EPOCH + std::time::Duration::from_secs(1_783_081_496)),
            "2026-07-03T12:24:56Z"
        );
        assert_eq!(
            utc_display_timestamp(UNIX_EPOCH + std::time::Duration::from_secs(1_783_081_496)),
            "07/03/2026 12:24 PM (UTC)"
        );
    }
}
