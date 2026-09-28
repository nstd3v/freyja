mod cli;
mod commands;
mod helpers;

use clap::Parser;

use cli::{Cli, Command};
use freyja_builder_podman::BuildkitBuilder;
use freyja_core::{error::Error, planner::Planner, resolver::ResolverRegistry};
// use freyja_extension_altrpm::AltRpmResolver;
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

    let cache = cli.freyja_dir.join("cache").join("alt-rpm");
    //resolvers.register(AltRpmResolver::new(cache));

    let planner = Planner::new(&resolvers);
    let builder = BuildkitBuilder::new();

    match cli.command {
        Command::Plan(_args) => {
            commands::plan(&cli.file, &cli.state, &planner).await?;
        }

        Command::Build(_args) => {
            commands::build(&cli.file, &cli.state, &planner, &builder).await?;
        }

        Command::Explain(args) => {
            commands::explain(&args.target, &cli.file, &cli.state, &planner).await?
        }
    }

    Ok(())
}
