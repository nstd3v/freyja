mod cli;
mod commands;

use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<(), freyja_core::error::Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::Plan(args) => commands::plan(&cli.file, args),
        Command::Build(args) => commands::build(&cli.file, args),
        Command::Explain(args) => commands::explain(&cli.file, args),
    }
}
