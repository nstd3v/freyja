mod cli;
mod commands;
mod helpers;

use clap::Parser;

use crate::helpers::load_spec;
use cli::{Cli, Command};
use freyja_builder_podman::BuildkitBuilder;
use freyja_core::{error::Error, planner::Planner, resolver::ResolverRegistry};
use freyja_extension_altrpm::AltRpmResolver;
use freyja_extension_apk::ApkResolver;
use freyja_extension_deb::DebResolver;
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
    let spec = load_spec(&cli.file)?;

    let mut resolvers = ResolverRegistry::new();
    if spec.extensions.oci.as_ref().is_some_and(|ext| ext.enabled) {
        resolvers.register(OciResolver::new());
    }
    if spec
        .extensions
        .alt_rpm
        .as_ref()
        .is_some_and(|ext| ext.enabled)
    {
        let cache = cli.freyja_dir.join("cache").join("alt-rpm");
        resolvers.register(AltRpmResolver::new(cache));
    }
    if spec.extensions.apk.as_ref().is_some_and(|ext| ext.enabled) {
        resolvers.register(ApkResolver::new(cli.freyja_dir.join("cache").join("apk")));
    }
    if spec.extensions.deb.as_ref().is_some_and(|ext| ext.enabled) {
        resolvers.register(DebResolver::new(cli.freyja_dir.join("cache").join("deb")));
    }

    let planner = Planner::new(&resolvers);
    let builder = BuildkitBuilder::new();

    match cli.command {
        Command::Plan(_args) => {
            commands::plan(&spec, &cli.state, &planner).await?;
        }

        Command::Build(_args) => {
            commands::build(&spec, &cli.state, &planner, &builder).await?;
        }

        Command::Explain(args) => {
            commands::explain(&args.target, &spec, &cli.state, &planner).await?
        }
    }

    Ok(())
}
