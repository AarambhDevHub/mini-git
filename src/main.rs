use clap::{Parser, Subcommand};
use mini_git::{Result, commands, utils};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mini_git")]
#[command(about = "A mini Git implementation in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(help = "Directory to initialize")]
        path: Option<PathBuf>,
    },
    Add {
        #[arg(help = "Files to add")]
        files: Vec<String>,
    },
    Commit {
        #[arg(short, long, help = "Commit message")]
        message: String,
        #[arg(short, long, help = "Author")]
        author: Option<String>,
    },
    Status,
    Log {
        #[arg(short, long, help = "Maximum number of commits to show")]
        max_count: Option<usize>,
    },
    Branch {
        #[arg(help = "Branch name")]
        name: Option<String>,
        #[arg(short, long, help = "Delete branch")]
        delete: bool,
    },
    Checkout {
        #[arg(help = "Branch or commit to checkout")]
        target: String,
    },
    Clone {
        #[arg(help = "Repository URL to clone")]
        url: String,
        #[arg(help = "Directory name")]
        directory: Option<String>,
    },
    Diff {
        #[arg(help = "Files to diff")]
        files: Vec<String>,
    },
    Merge {
        #[arg(help = "Branch to merge")]
        branch: String,
        #[arg(short, long, help = "Author")]
        author: Option<String>,
    },
    Push {
        #[arg(help = "Remote name")]
        remote: Option<String>,
        #[arg(help = "Branch name")]
        branch: Option<String>,
    },
    Pull {
        #[arg(help = "Remote name")]
        remote: Option<String>,
        #[arg(help = "Branch name")]
        branch: Option<String>,
    },
    Remote {
        #[arg(help = "Action: add, remove, set-url, get-url, -v")]
        action: Option<String>,
        #[arg(help = "Remote name")]
        name: Option<String>,
        #[arg(help = "Remote URL")]
        url: Option<String>,
    },
    Stash {
        #[arg(help = "Action: push, pop, list, show, drop, clear")]
        action: Option<String>,
        #[arg(short, long, help = "Stash message")]
        message: Option<String>,
        #[arg(short, long, help = "Stash index")]
        index: Option<usize>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            commands::init(path)?;
        }
        Commands::Clone { url, directory } => {
            commands::clone(url, directory)?;
        }
        _ => {
            let repo = utils::get_repository(None)?;

            match cli.command {
                Commands::Add { files } => {
                    commands::add(&repo, files)?;
                }
                Commands::Commit { message, author } => {
                    commands::commit(&repo, message, author)?;
                }
                Commands::Status => {
                    commands::status(&repo)?;
                }
                Commands::Log { max_count } => {
                    commands::log(&repo, max_count)?;
                }
                Commands::Branch { name, delete } => {
                    commands::branch(&repo, name, delete)?;
                }
                Commands::Checkout { target } => {
                    commands::checkout(&repo, target)?;
                }
                Commands::Diff { files } => {
                    commands::diff(&repo, files)?;
                }
                Commands::Merge { branch, author } => {
                    commands::merge(&repo, branch, author)?;
                }
                Commands::Push { remote, branch } => {
                    commands::push(&repo, remote, branch)?;
                }
                Commands::Pull { remote, branch } => {
                    commands::pull(&repo, remote, branch)?;
                }
                Commands::Remote { action, name, url } => {
                    commands::remote(&repo, action, name, url)?;
                }
                Commands::Stash {
                    action,
                    message,
                    index,
                } => {
                    commands::stash(&repo, action, message, index)?;
                }
                Commands::Init { .. } | Commands::Clone { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}
