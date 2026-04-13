mod config;
mod create;
mod freeze;
mod manifest;
mod meta;
mod nuget;
mod publish;
mod remove;
mod spinner;
mod upgrade;
mod upm;
mod versions;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use config::{
    add_editor, add_registry, list_editors, list_registries, remove_editor, remove_registry,
    scan_and_merge_editors, set_default_editor, set_default_registry, set_origin_registry_token,
    RegistryKind,
};
use manifest::{dependencies_string_map, load_manifest_value, MANIFEST_PATH};
use reqwest::Client;
use std::path::Path;

#[derive(Parser)]
#[command(name = "uupm", version, about = "Unity Package Manager CLI (Rust)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package from UPM registry (semver in manifest), or from NuGet with -n
    #[command(alias = "i")]
    Install {
        name: String,
        #[arg(long, short = 'n')]
        nuget: bool,
        /// Download UPM package as .tgz into Packages/ and set dependency to file:… (not for NuGet)
        #[arg(long, conflicts_with = "nuget")]
        embed: bool,
        /// NuGet source name from ~/.upmrc (optional)
        source: Option<String>,
    },
    /// Remove a package from manifest and clean up local artifacts
    #[command(alias = "rm")]
    Remove { name: String },
    /// List packages in manifest
    #[command(alias = "ls")]
    List,
    /// Upgrade a package (or all packages) to the latest registry version
    #[command(alias = "up")]
    Upgrade {
        /// Package name to upgrade; omit to upgrade all
        name: Option<String>,
    },
    /// Create a new Unity package scaffold in the current directory
    #[command(alias = "c")]
    Create {
        /// Reverse-domain package name, e.g. com.vendor.mylib
        name: String,
        /// Display name (defaults to last segment, title-cased)
        #[arg(long)]
        display_name: Option<String>,
        /// Author name
        #[arg(long)]
        author: Option<String>,
        /// Initial version (default: 0.1.0)
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },
    /// Publish the package in the given directory to a UPM registry
    #[command(alias = "p")]
    Publish {
        /// Path to the package directory (default: current directory)
        #[arg(default_value = ".")]
        dir: String,
        /// Registry name from ~/.upmrc (defaults to scope-matched registry)
        #[arg(long, short = 'r')]
        registry: Option<String>,
    },
    /// Freeze manifest dependencies to local tarballs / embedded packages
    #[command(alias = "f")]
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
        /// Scope prefixes this registry handles, e.g. --scopes com.unity --scopes com.myco
        #[arg(long, num_args = 0..)]
        scopes: Vec<String>,
        /// Bearer token for this Unity registry (publish / authenticated APIs); not used with -n
        #[arg(long)]
        token: Option<String>,
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
    /// Set default UPM or NuGet registry name (must exist in sources)
    Default {
        name: String,
        #[arg(long, short = 'n')]
        nuget: bool,
    },
    /// Set or clear Bearer token on a Unity registry
    Token {
        name: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long, conflicts_with = "token")]
        clear: bool,
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
    /// Set default editor (version key from `editor list`)
    Default { name: String },
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
            embed,
            source,
        }) => {
            if nuget {
                nuget::install_nuget_package(&client, &name, source.as_deref()).await?;
                println!("Install finished!");
            } else {
                println!("Installing UPM package: {name}...");
                upm::install_upm_package(&client, &name, embed).await?;
                println!("Install finished!");
            }
        }
        Some(Commands::Remove { name }) => {
            remove::remove_package(&name)?;
        }
        Some(Commands::List) => {
            list_packages()?;
        }
        Some(Commands::Upgrade { name }) => {
            upgrade::upgrade_packages(&client, name.as_deref()).await?;
            println!("Upgrade finished!");
        }
        Some(Commands::Create {
            name,
            display_name,
            author,
            version,
        }) => {
            create::create_package(&name, display_name.as_deref(), author.as_deref(), &version)?;
        }
        Some(Commands::Publish { dir, registry }) => {
            publish::publish_package(&client, &dir, registry.as_deref()).await?;
        }
        Some(Commands::Freeze) => {
            println!("Freezing project packages...");
            freeze::freeze_packages(&client).await?;
            println!("Freeze finished!");
        }
        Some(Commands::Registry(sub)) => match sub {
            RegistryCli::Add {
                name,
                url,
                nuget,
                scopes,
                token,
            } => {
                let kind = if nuget {
                    RegistryKind::Nuget
                } else {
                    RegistryKind::Origin
                };
                add_registry(&name, &url, scopes, kind, token.as_deref())?;
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
            RegistryCli::Default { name, nuget } => {
                let kind = if nuget {
                    RegistryKind::Nuget
                } else {
                    RegistryKind::Origin
                };
                set_default_registry(&name, kind)?;
            }
            RegistryCli::Token { name, token, clear } => {
                if clear {
                    set_origin_registry_token(&name, None)?;
                } else if let Some(t) = token {
                    set_origin_registry_token(&name, Some(t.as_str()))?;
                } else {
                    bail!("specify --token <value> or --clear");
                }
            }
        },
        Some(Commands::Editor(sub)) => match sub {
            EditorCli::Scan => scan_and_merge_editors()?,
            EditorCli::Add { name, path } => add_editor(&name, &path)?,
            EditorCli::Remove { name } => remove_editor(&name)?,
            EditorCli::List => list_editors()?,
            EditorCli::Default { name } => set_default_editor(&name)?,
        },
    }

    Ok(())
}

fn list_packages() -> Result<()> {
    if !Path::new(MANIFEST_PATH).exists() {
        println!("No {} found.", MANIFEST_PATH);
        return Ok(());
    }
    let manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let deps = dependencies_string_map(&manifest_v);
    if deps.is_empty() {
        println!("No dependencies.");
        return Ok(());
    }
    let name_w = deps.keys().map(|k| k.len()).max().unwrap_or(0);
    for (name, version) in &deps {
        let kind = if version.starts_with("file:") {
            "local"
        } else if version.starts_with("git:") || version.starts_with("https://") {
            "git"
        } else {
            "registry"
        };
        println!("{:<width$}  {}  ({})", name, version, kind, width = name_w);
    }
    Ok(())
}
