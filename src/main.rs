use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use directories::UserDirs;
use glob::glob;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

static PROGNAME: &'static str = env!("CARGO_PKG_NAME");

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    conf: PathBuf,
    editor: String,
    notes_dir: PathBuf,
    extension: String,
    viewer: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(
    next_line_help = true,
    propagate_version = true,
    arg_required_else_help = true
)]
struct Cli {
    /// Show current configuration
    #[arg(short, long)]
    conf: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new note
    New(NoteArgs),

    /// List notes
    Ls,

    /// View note content
    Cat(NoteArgs),

    /// Find note by name
    Find(NoteArgs),

    /// Grep in notes content
    Grep(NoteArgs),

    /// Open the note in the editor
    Open(NoteArgs),
}

#[derive(Args)]
struct NoteArgs {
    note_arg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: "vim".into(),
            extension: "md".into(),
            viewer: "cat".into(),
            conf: confy::get_configuration_file_path(PROGNAME, PROGNAME)
                .expect("Failed to get configuration file"),
            notes_dir: UserDirs::new()
                .expect("Failed to get user directory")
                .home_dir()
                .join(PROGNAME)
                .to_path_buf(),
        }
    }
}

fn do_ls(notes: &HashMap<String, PathBuf>) -> ! {
    let mut sorted: Vec<_> = notes.iter().collect();

    sorted.sort_by_key(|k| k.0);

    println!("");
    for (note, _) in sorted {
        println!("{}", note.bold());
    }

    std::process::exit(exitcode::OK);
}

fn _cmd<P, N>(cmd: &str, path: P, note: N) -> Result<()>
where
    P: AsRef<Path>,
    N: AsRef<OsStr>,
{
    let v: Vec<&str> = cmd.split(' ').collect();

    Command::new(v[0])
        .current_dir(path)
        .args(v.iter().skip(1))
        .arg(note)
        .status()
        .context("Failed to open viewer or editor")?;

    Ok(())
}

fn do_new(note_name: &str, cfg: &Config) -> Result<()> {
    fs::create_dir_all(&cfg.notes_dir)?;

    let note_file = Path::new(&note_name).with_extension(&cfg.extension);

    _cmd(&cfg.editor, &cfg.notes_dir, &note_file)?;

    println!("");
    println!(
        "Created {} in {}",
        format!("{}", note_file.display()).bold(),
        format!("{}", cfg.notes_dir.display()).bold()
    );

    Ok(())
}

fn do_cmd(
    note_name: &str,
    notes: &HashMap<String, PathBuf>,
    cfg: &Config,
    cmd: &str,
) -> Result<()> {
    if let Some(n) = notes.get(note_name) {
        _cmd(cmd, &cfg.notes_dir, &n)?;
    } else {
        eprintln!("{note_name} not found");
        std::process::exit(exitcode::UNAVAILABLE);
    }

    Ok(())
}

fn do_find(note_arg: &str, notes: &HashMap<String, PathBuf>) {
    println!("");
    notes
        .iter()
        .filter(|x| x.0.contains(&note_arg))
        .for_each(|x| println!("{}", x.0.bold()));
}

fn do_grep(note_arg: &str, notes: &HashMap<String, PathBuf>) -> Result<()> {
    for (note, path) in notes {
        let mut nl: bool = true;

        fs::read_to_string(path)?
            .lines()
            .filter(|l| l.contains(&note_arg))
            .for_each(move |l| {
                if nl == true {
                    println!("");
                    nl = false;
                }
                println!("{}: {}", note.bold(), l);
            });
    }

    Ok(())
}

fn build_notes(cfg: &Config) -> Result<HashMap<String, PathBuf>> {
    let notes = cfg
        .notes_dir
        .join("*")
        .with_extension(&cfg.extension)
        .into_os_string();

    let c = glob(&notes.to_string_lossy())?
        .map(|p| {
            let note_path = p?;
            let note_name = note_path
                .file_stem()
                .context("Failed to get file stem")?
                .to_str()
                .context("Failed to get file name")?
                .to_owned();
            Ok((note_name, note_path))
        })
        .collect::<Result<Vec<(String, PathBuf)>>>();

    let m: HashMap<String, PathBuf> = HashMap::from_iter(c?);

    Ok(m)
}

fn do_print_config(cfg: &Config) -> ! {
    println!("");
    println!(
        "Configuration file: \t{}",
        format!("{}", cfg.conf.display()).bold()
    );
    println!(
        "Notes directory: \t{}",
        format!("{}", cfg.notes_dir.display()).bold()
    );
    println!("Editor: \t\t{}", cfg.editor.bold());
    println!("Viewer: \t\t{}", cfg.viewer.bold());
    println!("Notes extension: \t{}", cfg.extension.bold());

    std::process::exit(exitcode::OK);
}

fn main() -> Result<()> {
    let cfg = confy::load(PROGNAME, PROGNAME)?;
    let cli = Cli::parse();

    if cli.conf {
        do_print_config(&cfg);
    }

    let notes = build_notes(&cfg)?;

    match cli.command {
        Some(Commands::New(arg)) => {
            do_new(&arg.note_arg, &cfg)?;
        }

        Some(Commands::Cat(arg)) => {
            do_cmd(&arg.note_arg, &notes, &cfg, &cfg.viewer)?;
        }

        Some(Commands::Open(arg)) => {
            do_cmd(&arg.note_arg, &notes, &cfg, &cfg.editor)?;
        }

        Some(Commands::Ls) => {
            do_ls(&notes);
        }

        Some(Commands::Find(arg)) => {
            do_find(&arg.note_arg, &notes);
        }

        Some(Commands::Grep(arg)) => {
            do_grep(&arg.note_arg, &notes)?;
        }

        _ => {}
    }

    Ok(())
}
