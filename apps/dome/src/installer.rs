//! Install Dome binaries and their agent skills.

use std::env;
use std::fmt;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use console::Style;
use dialoguer::{
    Input, MultiSelect,
    theme::{ColorfulTheme, Theme},
};
use sha2::{Digest, Sha256};

const GITHUB_REPOSITORY: &str = "bathan1/dome";
const RELEASE_TARGET: &str = "x86_64-unknown-linux-gnu";
const MANAGED_MARKER: &str = ".dome-managed";
#[cfg(test)]
const CLIPME_SKILL: &str = include_str!("../../clipme/skill/SKILL.md");
#[cfg(test)]
const CLIPME_OPENAI_YAML: &str = include_str!("../../clipme/skill/agents/openai.yaml");

struct ReleaseSkill {
    markdown: String,
    openai_yaml: String,
}

#[derive(Default)]
struct AgentSelectionTheme {
    base: ColorfulTheme,
}

impl Theme for AgentSelectionTheme {
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.base.format_multi_select_prompt(f, prompt)
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        self.base
            .format_multi_select_prompt_selection(f, prompt, selections)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        if active {
            write!(
                f,
                "{} ",
                Style::new().for_stderr().cyan().bold().apply_to("❯")
            )?;
        } else {
            write!(f, "  ")?;
        }

        let checkbox = if checked {
            &self.base.checked_item_prefix
        } else {
            &self.base.unchecked_item_prefix
        };
        let item = if active {
            self.base.active_item_style.apply_to(text)
        } else {
            self.base.inactive_item_style.apply_to(text)
        };

        write!(f, "{checkbox} {item}")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Add,
    Remove,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Command {
    operation: Operation,
    binary: String,
}

impl Command {
    pub fn parse(arguments: impl Iterator<Item = String>) -> io::Result<Self> {
        let arguments: Vec<_> = arguments.collect();
        let [operation, binary] = arguments.as_slice() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected an operation and binary name",
            ));
        };

        let operation = match operation.as_str() {
            "add" => Operation::Add,
            "remove" => Operation::Remove,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown operation `{other}`; expected `add` or `remove`"),
                ));
            }
        };

        if !matches!(binary.as_str(), "clipme" | "squid") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown binary `{binary}`; available binaries: clipme, squid"),
            ));
        }

        Ok(Self {
            operation,
            binary: binary.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Environment {
    home: PathBuf,
    binary_directory: PathBuf,
    state_directory: PathBuf,
}

impl Environment {
    pub fn from_process() -> io::Result<Self> {
        if env::consts::OS != "linux" || env::consts::ARCH != "x86_64" {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "this release currently supports x86_64 Linux and WSL only",
            ));
        }

        let home = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        let cargo_home = env::var_os("CARGO_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cargo"));
        let state_directory = env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"))
            .join("dome");

        Ok(Self {
            home,
            binary_directory: cargo_home.join("bin"),
            state_directory,
        })
    }

    #[cfg(test)]
    fn for_test(root: &Path) -> Self {
        Self {
            home: root.join("home"),
            binary_directory: root.join("bin"),
            state_directory: root.join("state"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Agent {
    Codex,
    ClaudeCode,
}

impl Agent {
    fn key(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Codex => "\x1b[97mCodex\x1b[0m",
            Self::ClaudeCode => "\x1b[38;2;217;119;87mClaude Code\x1b[0m",
        }
    }

    fn default_skills_root(self) -> &'static str {
        match self {
            Self::Codex => "~/.agents/skills",
            Self::ClaudeCode => "~/.claude/skills",
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Codex => "Codex skills directory",
            Self::ClaudeCode => "Claude Code skills directory",
        }
    }
}

pub fn run(command: Command, environment: &Environment) -> io::Result<()> {
    match command.operation {
        Operation::Add => add(&command.binary, environment),
        Operation::Remove => remove(&command.binary, environment),
    }
}

fn add(binary: &str, environment: &Environment) -> io::Result<()> {
    let binary_path = environment.binary_directory.join(binary);
    let release_skill = install_latest_release(binary, &binary_path)?;
    write_state_path(environment, binary, "binary", &binary_path)?;

    let Some(release_skill) = release_skill else {
        return Ok(());
    };

    if !io::stdin().is_terminal() {
        return Err(io::Error::other(format!(
            "installed {} but agent selection needs an interactive terminal; rerun `dome add {binary}` interactively",
            binary_path.display()
        )));
    }

    let agents = [Agent::Codex, Agent::ClaudeCode];
    let labels: Vec<_> = agents.iter().map(|agent| agent.label()).collect();
    let selected = MultiSelect::with_theme(&AgentSelectionTheme::default())
        .with_prompt(format!("Install the {binary} skill for"))
        .items(&labels)
        .interact()
        .map_err(dialoguer_error)?;

    for index in selected {
        let agent = agents[index];
        let root: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(agent.prompt())
            .default(agent.default_skills_root().to_owned())
            .interact_text()
            .map_err(dialoguer_error)?;
        let skill_directory = expand_home(&root, &environment.home)?.join(binary);
        install_skill(
            binary,
            &skill_directory,
            &release_skill.markdown,
            &release_skill.openai_yaml,
        )?;
        write_state_path(environment, binary, agent.key(), &skill_directory)?;
        println!(
            "{} {}",
            Style::new().green().apply_to("installed skill:"),
            skill_directory.display()
        );
    }

    Ok(())
}

fn remove(binary: &str, environment: &Environment) -> io::Result<()> {
    let recorded_binary = read_state_path(environment, binary, "binary")?
        .unwrap_or_else(|| environment.binary_directory.join(binary));
    let binary_existed = recorded_binary.exists();
    remove_file_idempotently(&recorded_binary)?;
    println!(
        "{} {}",
        Style::new().yellow().apply_to(if binary_existed {
            "removed binary:"
        } else {
            "binary already absent:"
        }),
        recorded_binary.display()
    );

    if app_has_skill(binary) {
        for agent in [Agent::Codex, Agent::ClaudeCode] {
            let skill_directory = read_state_path(environment, binary, agent.key())?
                .unwrap_or_else(|| {
                    expand_home(agent.default_skills_root(), &environment.home)
                        .expect("built-in paths are valid")
                        .join(binary)
                });
            if remove_managed_skill(binary, &skill_directory)? {
                println!(
                    "{} {}",
                    Style::new().yellow().apply_to("removed skill:"),
                    skill_directory.display()
                );
            }
        }
    }

    let state = binary_state_directory(environment, binary);
    if state.exists() {
        fs::remove_dir_all(state)?;
    }
    Ok(())
}

fn app_has_skill(binary: &str) -> bool {
    binary == "clipme"
}

fn install_latest_release(binary: &str, destination: &Path) -> io::Result<Option<ReleaseSkill>> {
    let base_url = env::var("DOME_RELEASE_BASE_URL").unwrap_or_else(|_| {
        format!("https://github.com/{GITHUB_REPOSITORY}/releases/latest/download")
    });
    install_release_from_base(binary, destination, &base_url)
}

fn install_release_from_base(
    binary: &str,
    destination: &Path,
    base_url: &str,
) -> io::Result<Option<ReleaseSkill>> {
    fs::create_dir_all(destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "binary destination has no parent",
        )
    })?)?;

    let asset = format!("{binary}-{RELEASE_TARGET}");
    let skill_asset = format!("{binary}-SKILL.md");
    let openai_yaml_asset = format!("{binary}-openai.yaml");
    let temporary = temporary_path(destination, "download");
    let skill_temporary = temporary_path(destination, "skill");
    let openai_yaml_temporary = temporary_path(destination, "openai-yaml");
    let checksums = temporary_path(destination, "checksums");

    let result = (|| {
        download(&format!("{base_url}/SHA256SUMS"), &checksums)?;
        download(&format!("{base_url}/{asset}"), &temporary)?;
        verify_checksum(&temporary, &checksums, &asset)?;
        set_executable(&temporary)?;

        let release_skill = if app_has_skill(binary) {
            download(&format!("{base_url}/{skill_asset}"), &skill_temporary)?;
            download(
                &format!("{base_url}/{openai_yaml_asset}"),
                &openai_yaml_temporary,
            )?;
            verify_checksum(&skill_temporary, &checksums, &skill_asset)?;
            verify_checksum(&openai_yaml_temporary, &checksums, &openai_yaml_asset)?;
            Some(ReleaseSkill {
                markdown: fs::read_to_string(&skill_temporary)?,
                openai_yaml: fs::read_to_string(&openai_yaml_temporary)?,
            })
        } else {
            None
        };

        if files_equal(&temporary, destination)? {
            fs::remove_file(&temporary)?;
            println!(
                "{} {}",
                Style::new().green().apply_to("binary already current:"),
                destination.display()
            );
        } else {
            replace_file(&temporary, destination)?;
            println!(
                "{} {}",
                Style::new().green().apply_to("installed binary:"),
                destination.display()
            );
        }
        Ok(release_skill)
    })();

    let _ = fs::remove_file(&temporary);
    let _ = fs::remove_file(&skill_temporary);
    let _ = fs::remove_file(&openai_yaml_temporary);
    let _ = fs::remove_file(&checksums);
    result
}

fn download(url: &str, destination: &Path) -> io::Result<()> {
    let status = ProcessCommand::new("curl")
        .args(["--fail", "--location", "--silent", "--show-error"])
        .arg("--output")
        .arg(destination)
        .arg(url)
        .status()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("could not run curl to download {url}: {error}"),
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "download failed for {url} with {status}"
        )))
    }
}

fn verify_checksum(binary: &Path, checksum_file: &Path, asset: &str) -> io::Result<()> {
    let checksum_text = fs::read_to_string(checksum_file)?;
    let expected = checksum_text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.next()?.trim_start_matches('*')))
        })
        .find_map(|(checksum, name)| (name == asset).then_some(checksum))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("SHA256SUMS does not contain {asset}"),
            )
        })?;

    let actual = Sha256::digest(fs::read(binary)?);
    let actual_hex = format!("{actual:x}");
    if actual_hex == expected.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("checksum mismatch for {asset}"),
        ))
    }
}

fn install_skill(
    binary: &str,
    directory: &Path,
    skill_markdown: &str,
    openai_yaml: &str,
) -> io::Result<()> {
    let skill_file = directory.join("SKILL.md");
    let marker = directory.join(MANAGED_MARKER);
    if skill_file.exists() && !marker.exists() {
        if fs::read_to_string(&skill_file)? == skill_markdown {
            fs::write(&marker, managed_marker(binary))?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to overwrite unmanaged skill at {}; choose another skills directory or remove it first",
                    skill_file.display()
                ),
            ));
        }
    }

    fs::create_dir_all(directory.join("agents"))?;
    atomic_write(&skill_file, skill_markdown.as_bytes())?;
    atomic_write(
        &directory.join("agents/openai.yaml"),
        openai_yaml.as_bytes(),
    )?;
    atomic_write(&marker, managed_marker(binary).as_bytes())?;
    Ok(())
}

fn remove_managed_skill(binary: &str, directory: &Path) -> io::Result<bool> {
    if !directory.exists() {
        return Ok(false);
    }
    let marker = directory.join(MANAGED_MARKER);
    if !marker.exists() || fs::read_to_string(&marker)? != managed_marker(binary) {
        return Ok(false);
    }

    remove_file_idempotently(&directory.join("SKILL.md"))?;
    remove_file_idempotently(&directory.join("agents/openai.yaml"))?;
    remove_file_idempotently(&marker)?;
    remove_directory_if_empty(&directory.join("agents"))?;
    remove_directory_if_empty(directory)?;
    Ok(true)
}

fn managed_marker(binary: &str) -> String {
    format!("managed-by=dome\nbinary={binary}\n")
}

fn binary_state_directory(environment: &Environment, binary: &str) -> PathBuf {
    environment.state_directory.join(binary)
}

fn write_state_path(
    environment: &Environment,
    binary: &str,
    key: &str,
    path: &Path,
) -> io::Result<()> {
    let state_file = binary_state_directory(environment, binary).join(key);
    atomic_write(&state_file, path.to_string_lossy().as_bytes())
}

fn read_state_path(
    environment: &Environment,
    binary: &str,
    key: &str,
) -> io::Result<Option<PathBuf>> {
    let state_file = binary_state_directory(environment, binary).join(key);
    match fs::read_to_string(state_file) {
        Ok(path) => Ok(Some(PathBuf::from(path))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn expand_home(input: &str, home: &Path) -> io::Result<PathBuf> {
    if input == "~" {
        return Ok(home.to_path_buf());
    }
    if let Some(rest) = input.strip_prefix("~/") {
        return Ok(home.join(rest));
    }
    if input.starts_with('~') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "only `~` and `~/...` home paths are supported",
        ));
    }
    Ok(PathBuf::from(input))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = temporary_path(path, "write");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    replace_file(&temporary, path)
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn temporary_path(path: &Path, purpose: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dome");
    path.with_file_name(format!(".{file_name}.{purpose}.{}", std::process::id()))
}

fn files_equal(left: &Path, right: &Path) -> io::Result<bool> {
    if !right.exists() {
        return Ok(false);
    }
    Ok(fs::read(left)? == fs::read(right)?)
}

fn remove_file_idempotently(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_directory_if_empty(path: &Path) -> io::Result<()> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn dialoguer_error(error: dialoguer::Error) -> io::Error {
    io::Error::other(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(test_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!("dome-{test_name}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("temporary directory can be created");
        path
    }

    #[test]
    fn command_requires_a_known_operation_and_binary() {
        let command =
            Command::parse(["add", "clipme"].map(String::from).into_iter()).expect("valid command");
        assert_eq!(command.operation, Operation::Add);
        assert_eq!(command.binary, "clipme");

        let squid =
            Command::parse(["add", "squid"].map(String::from).into_iter()).expect("valid command");
        assert_eq!(squid.binary, "squid");

        assert!(Command::parse(["update", "clipme"].map(String::from).into_iter()).is_err());
        assert!(Command::parse(["add", "unknown"].map(String::from).into_iter()).is_err());
        assert!(Command::parse(["add"].map(String::from).into_iter()).is_err());
    }

    #[test]
    fn agent_selection_cursor_stays_visible_for_checked_items() {
        let theme = AgentSelectionTheme::default();

        for checked in [false, true] {
            let mut active = String::new();
            theme
                .format_multi_select_prompt_item(&mut active, "Codex", checked, true)
                .unwrap();
            assert!(console::strip_ansi_codes(&active).starts_with("❯ "));

            let mut inactive = String::new();
            theme
                .format_multi_select_prompt_item(&mut inactive, "Codex", checked, false)
                .unwrap();
            assert!(console::strip_ansi_codes(&inactive).starts_with("  "));
        }
    }

    #[test]
    fn home_paths_are_expanded_without_expanding_other_users() {
        let home = Path::new("/tmp/example-home");
        assert_eq!(expand_home("~", home).unwrap(), home);
        assert_eq!(
            expand_home("~/.agents/skills", home).unwrap(),
            home.join(".agents/skills")
        );
        assert!(expand_home("~someone/skills", home).is_err());
    }

    #[test]
    fn skill_installation_is_idempotent_and_removable() {
        let root = temporary_directory("skill-lifecycle");
        let skill = root.join("skills/clipme");

        install_skill("clipme", &skill, CLIPME_SKILL, CLIPME_OPENAI_YAML).unwrap();
        install_skill("clipme", &skill, CLIPME_SKILL, CLIPME_OPENAI_YAML).unwrap();
        assert_eq!(
            fs::read_to_string(skill.join("SKILL.md")).unwrap(),
            CLIPME_SKILL
        );
        assert!(remove_managed_skill("clipme", &skill).unwrap());
        assert!(!remove_managed_skill("clipme", &skill).unwrap());
        assert!(!skill.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn skill_installation_does_not_overwrite_an_unmanaged_skill() {
        let root = temporary_directory("unmanaged-skill");
        let skill = root.join("skills/clipme");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "personal instructions").unwrap();

        let error = install_skill("clipme", &skill, CLIPME_SKILL, CLIPME_OPENAI_YAML).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(skill.join("SKILL.md")).unwrap(),
            "personal instructions"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn state_paths_round_trip() {
        let root = temporary_directory("state");
        let environment = Environment::for_test(&root);
        let path = environment.home.join("skills with spaces/clipme");

        write_state_path(&environment, "clipme", "codex", &path).unwrap();
        assert_eq!(
            read_state_path(&environment, "clipme", "codex").unwrap(),
            Some(path)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checksum_verification_finds_the_named_release_asset() {
        let root = temporary_directory("checksum");
        let binary = root.join("clipme");
        let checksums = root.join("SHA256SUMS");
        fs::write(&binary, b"release bytes").unwrap();
        let digest = Sha256::digest(b"release bytes");
        fs::write(&checksums, format!("{digest:x}  clipme-{RELEASE_TARGET}\n")).unwrap();

        verify_checksum(&binary, &checksums, &format!("clipme-{RELEASE_TARGET}")).unwrap();

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn release_installation_downloads_verifies_and_reuses_current_bytes() {
        let root = temporary_directory("release-install");
        let release = root.join("release");
        let destination = root.join("bin/clipme");
        let asset = format!("clipme-{RELEASE_TARGET}");
        let skill_asset = "clipme-SKILL.md";
        let openai_yaml_asset = "clipme-openai.yaml";
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join(&asset), b"published clipme").unwrap();
        fs::write(release.join(skill_asset), CLIPME_SKILL).unwrap();
        fs::write(release.join(openai_yaml_asset), CLIPME_OPENAI_YAML).unwrap();
        let binary_digest = Sha256::digest(b"published clipme");
        let skill_digest = Sha256::digest(CLIPME_SKILL);
        let openai_yaml_digest = Sha256::digest(CLIPME_OPENAI_YAML);
        fs::write(
            release.join("SHA256SUMS"),
            format!(
                "{binary_digest:x}  {asset}\n{skill_digest:x}  {skill_asset}\n{openai_yaml_digest:x}  {openai_yaml_asset}\n"
            ),
        )
        .unwrap();
        let base_url = format!("file://{}", release.display());

        let first = install_release_from_base("clipme", &destination, &base_url).unwrap();
        let second = install_release_from_base("clipme", &destination, &base_url).unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"published clipme");
        assert_eq!(first.unwrap().markdown, CLIPME_SKILL);
        assert_eq!(second.unwrap().openai_yaml, CLIPME_OPENAI_YAML);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn release_installation_supports_apps_without_skills() {
        let root = temporary_directory("release-without-skill");
        let release = root.join("release");
        let destination = root.join("bin/squid");
        let asset = format!("squid-{RELEASE_TARGET}");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join(&asset), b"published squid").unwrap();
        let digest = Sha256::digest(b"published squid");
        fs::write(release.join("SHA256SUMS"), format!("{digest:x}  {asset}\n")).unwrap();
        let base_url = format!("file://{}", release.display());

        let skill = install_release_from_base("squid", &destination, &base_url).unwrap();
        assert!(skill.is_none());
        assert_eq!(fs::read(destination).unwrap(), b"published squid");

        fs::remove_dir_all(root).unwrap();
    }
}
