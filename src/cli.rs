use clap::{CommandFactory, Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};

use crate::commands;
use crate::terminal_editor;
use crate::{editor::editor_for_file, file::read_file_for_editing};

#[derive(Parser, Debug)]
#[command(name = "kern", version, about = "A terminal text editor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// File to open
    pub path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Update kern to latest release
    Update {
        /// Check for updates without installing them
        #[arg(long)]
        check: bool,
    },
}

impl Cli {
    pub fn run(self) -> ExitCode {
        match self.command {
            Some(Commands::Update { check }) => commands::update::run_update(check),
            None => process_kern_default_command(self.path),
        }
    }
}

fn process_kern_default_command(path: Option<PathBuf>) -> ExitCode {
    match path {
        Some(path) => {
            let contents = match read_file_for_editing(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    eprintln!("error: cannot open {}: {error}", path.display());
                    return ExitCode::FAILURE;
                }
            };
            let editor = editor_for_file(&contents);

            match terminal_editor::run(editor, path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("Terminal error: {error}");
                    ExitCode::FAILURE
                }
            }
        }

        None => {
            let error = Cli::command().error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "a file path is required",
            );
            eprint!("{error}");
            ExitCode::FAILURE
        }
    }
}
