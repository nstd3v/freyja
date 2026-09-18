mod cli;
mod commands;
mod helpers;

use clap::Parser;

use cli::{Cli, Command};
use freyja_core::{error::Error, planner::Planner, resolver::ResolverRegistry};
use freyja_extension_oci::OciResolver;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    let mut resolvers = ResolverRegistry::new();
    resolvers.register(OciResolver::new());

    let planner = Planner::new(&resolvers);

    match cli.command {
        Command::Plan(_args) => {
            commands::plan(&cli.file, &cli.state, &planner).await?;
        }

        Command::Build(_) => {
            todo!("build command");
        }

        Command::Explain(_) => {
            todo!("explain command");
        }
    }

    Ok(())
}
