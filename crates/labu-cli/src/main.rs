use clap::{Parser, Subcommand};
use std::process::ExitCode;

/// labu: test bench reliability untuk agent.
#[derive(Parser)]
#[command(name = "labu", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// Hanya subcommand v0.1 yang tampil di --help (lihat plan bagian 5.1).
#[derive(Subcommand)]
enum Command {
    /// Rekam call LLM dari <command> ke cassette (belum diimplementasi)
    Record {
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
    /// Putar ulang dari cassette, offline (belum diimplementasi)
    Replay {
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let name = match cli.command {
        Command::Record { .. } => "record",
        Command::Replay { .. } => "replay",
    };
    eprintln!(
        "labu {name}: belum diimplementasi (labu-core {})",
        labu_core::version()
    );
    ExitCode::FAILURE
}
