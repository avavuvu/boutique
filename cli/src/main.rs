use std::{
    fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand};
use include_dir::{Dir, include_dir};
use minijinja::{Environment, context};

static TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/template");

const REPOSITORY: &str = "https://github.com/avavuvu/boutique.git";
const REV: &str = env!("BOUTIQUE_REV");
const IS_DIRTY: &str = env!("BOUTIQUE_DIRTY");
const JINJA: &str = ".jinja";

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    Bq(Bq),
}

#[derive(Args)]
#[command(version, about = "Boutique's command line")]
struct Bq {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New(New),
}

#[derive(Args)]
#[command(about = "Create a boutique app")]
struct New {
    path: PathBuf,
    #[arg(long, help = "Add sea-orm with Postgres")]
    database: bool,
    #[arg(long, help = "Add sign up, sign in and sign out (adds the database)")]
    accounts: bool,
    #[arg(long, short, help = "Take the defaults instead of asking")]
    yes: bool,
    #[arg(long, value_name = "DIR", help = "Depend on a local boutique checkout instead of git")]
    boutique_path: Option<PathBuf>,
}

struct Choices {
    database: bool,
    accounts: bool,
}

fn main() {
    let Cargo::Bq(bq) = Cargo::parse();
    let result = match bq.command {
        Command::New(new) => create(new),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn create(new: New) -> io::Result<()> {
    let name = new
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| is_valid_name(name))
        .ok_or_else(|| io::Error::other("use a name of lowercase letters, digits and hyphens that starts with a letter"))?
        .to_string();
    if new.path.exists() {
        return Err(io::Error::other(format!("{} already exists", new.path.display())));
    }

    let is_interactive = !new.yes && io::stdin().is_terminal();
    if is_interactive {
        cliclack::intro(format!("boutique: {name}"))?;
    }
    let choices = choose(&new, is_interactive)?;

    let (boutique, setups) = match &new.boutique_path {
        Some(path) => {
            let path = fs::canonicalize(path)?;
            (local(&path, "boutique"), local(&path, "setups"))
        }
        None => (git(), git()),
    };
    if new.boutique_path.is_none() && IS_DIRTY == "true" {
        eprintln!("warning: this cargo-bq was built from a boutique checkout with uncommitted changes; the project depends on {REV}");
    }

    let environment = environment();
    let values = context! {
        name => &name,
        database_name => name.replace('-', "_"),
        database => choices.database,
        accounts => choices.accounts,
        boutique => boutique,
        setups => setups,
    };

    let layers = ["base"]
        .into_iter()
        .chain(choices.database.then_some("database"))
        .chain(choices.accounts.then_some("accounts"));
    for layer in layers {
        let dir = TEMPLATE.get_dir(layer).expect("the template has every layer");
        write_dir(dir, Path::new(layer), &new.path, &environment, &values)?;
    }

    bq_setups::Generator::new(new.path.join("bindings"))
        .source(new.path.join("src"))
        .boutique()
        .try_write()
        .map_err(|error| io::Error::other(error.to_string()))?;

    let steps = next_steps(&new.path, &name, &choices);
    if is_interactive {
        cliclack::outro(steps)?;
    } else {
        println!("{steps}");
    }
    Ok(())
}

fn choose(new: &New, is_interactive: bool) -> io::Result<Choices> {
    let database = new.database
        || new.accounts
        || (is_interactive && cliclack::confirm("Add a Postgres database?").initial_value(false).interact()?);
    let accounts = new.accounts
        || (database && is_interactive && cliclack::confirm("Add accounts (sign up, sign in, sign out)?").initial_value(false).interact()?);
    Ok(Choices { database, accounts })
}

fn environment() -> Environment<'static> {
    let mut environment = Environment::new();
    environment.set_keep_trailing_newline(true);
    environment.set_trim_blocks(true);
    environment.set_lstrip_blocks(true);
    environment
}

fn write_dir(dir: &Dir, layer: &Path, out: &Path, environment: &Environment, values: &minijinja::Value) -> io::Result<()> {
    for file in dir.files() {
        let relative = file.path().strip_prefix(layer).expect("files sit under their layer");
        let source = relative.to_string_lossy();
        let target = out.join(source.strip_suffix(JINJA).unwrap_or(&source));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        match source.ends_with(JINJA) {
            true => {
                let text = file.contents_utf8().expect("templates are UTF-8");
                let rendered = environment
                    .render_str(text, values)
                    .map_err(|error| io::Error::other(format!("{source}: {error}")))?;
                fs::write(target, rendered)?;
            }
            false => fs::write(target, file.contents())?,
        }
    }
    for child in dir.dirs() {
        write_dir(child, layer, out, environment, values)?;
    }
    Ok(())
}

fn git() -> String {
    match REV {
        "" => format!("git = \"{REPOSITORY}\", branch = \"main\""),
        rev => format!("git = \"{REPOSITORY}\", rev = \"{rev}\""),
    }
}

fn local(root: &Path, crate_dir: &str) -> String {
    format!("path = \"{}\"", root.join(crate_dir).display())
}

fn is_valid_name(name: &str) -> bool {
    name.starts_with(|character: char| character.is_ascii_lowercase())
        && name.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-')
}

fn next_steps(path: &Path, name: &str, choices: &Choices) -> String {
    let mut steps = vec![format!("cd {}", path.display()), "cp .env.example .env".to_string()];
    if choices.database {
        steps.push(format!("createdb {}", name.replace('-', "_")));
    }
    steps.extend(["bun install".to_string(), "bun run build".to_string(), "bun run dev".to_string()]);
    format!("Next:\n  {}", steps.join("\n  "))
}
