mod config;
mod freeze;
mod manifest;
mod meta;
mod nuget;
mod spinner;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::{
    add_editor, add_registry, list_editors, list_registries, remove_editor, remove_registry,
    scan_and_merge_editors, RegistryKind,
};
use reqwest::Client;

#[derive(Parser)]
#[command(name = "uupm", version, about = "Unity Package Manager CLI (Rust)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package (UPM stub; use --nuget for NuGet)
    Install {
        name: String,
        #[arg(long, short = 'n')]
        nuget: bool,
        /// NuGet source name from ~/.upmrc (optional)
        source: Option<String>,
    },
    /// Freeze manifest dependencies to local tarballs / embedded packages
    Freeze,
    /// Manage registries
    #[command(subcommand, alias = "r")]
    Registry(RegistryCli),
    /// Manage Unity editor installations
    #[command(subcommand, alias = "e")]
    Editor(EditorCli),
}

#[derive(Subcommand)]
enum RegistryCli {
    /// Add a registry
    #[command(alias = "a")]
    Add {
        name: String,
        url: String,
        #[arg(long, short = 'n')]
        nuget: bool,
    },
    /// Remove a registry
    #[command(alias = "r")]
    Remove {
        name: String,
        #[arg(long, short = 'n')]
        nuget: bool,
    },
    /// List registries
    #[command(alias = "l")]
    List {
        #[arg(long, short = 'n')]
        nuget: bool,
    },
}

#[derive(Subcommand)]
enum EditorCli {
    /// Scan common install folders and merge into ~/.upmrc
    #[command(alias = "s")]
    Scan,
    /// Register an editor by name and path
    #[command(alias = "a")]
    Add { name: String, path: String },
    /// Remove an editor entry
    #[command(alias = "r")]
    Remove { name: String },
    /// List configured editors
    #[command(alias = "l")]
    List,
}

fn print_banner() {
    println!();
    println!(r" _   _ ____  ____  __  __ ");
    println!(r"| | | |  _ \|  _ \|  \/  |");
    println!(r"| |_| | |_) | |_) | |\/| |");
    println!(r" \___/| .__/| .__/|_|  |_|");
    println!(r"      |_|   |_|           ");
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::builder()
        .user_agent(concat!("uupm/", env!("CARGO_PKG_VERSION")))
        .build()?;

    match cli.command {
        None => print_banner(),
        Some(Commands::Install {
            name,
            nuget,
            source,
        }) => {
            if nuget {
                nuget::install_nuget_package(&client, &name, source.as_deref()).await?;
                println!("Install finished!");
            } else {
                println!("Installing package: {name}...");
                println!("UPM install is not implemented yet (original UnityPackageResolver was a stub).");
                println!("Use: uupm install -n {name} for NuGet packages.");
            }
        }
        Some(Commands::Freeze) => {
            println!("Freezing project packages...");
            freeze::freeze_packages(&client).await?;
            println!("Freeze finished!");
        }
        Some(Commands::Registry(sub)) => match sub {
            RegistryCli::Add { name, url, nuget } => {
                let kind = if nuget {
                    RegistryKind::Nuget
                } else {
                    RegistryKind::Origin
                };
                add_registry(&name, &url, kind)?;
            }
            RegistryCli::Remove { name, nuget } => {
                let kind = if nuget {
                    RegistryKind::Nuget
                } else {
                    RegistryKind::Origin
                };
                remove_registry(&name, kind)?;
            }
            RegistryCli::List { nuget } => {
                let kind = if nuget {
                    RegistryKind::Nuget
                } else {
                    RegistryKind::Origin
                };
                list_registries(kind)?;
            }
        },
        Some(Commands::Editor(sub)) => match sub {
            EditorCli::Scan => scan_and_merge_editors()?,
            EditorCli::Add { name, path } => add_editor(&name, &path)?,
            EditorCli::Remove { name } => remove_editor(&name)?,
            EditorCli::List => list_editors()?,
        },
    }

    Ok(())
}
