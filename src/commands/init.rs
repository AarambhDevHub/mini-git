use crate::{Repository, Result};
use std::fs;
use std::path::PathBuf;

pub fn init(path: Option<PathBuf>) -> Result<()> {
    let work_dir = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let git_dir = work_dir.join(".mini_git");

    if git_dir.exists() {
        return Err("Repository already exists".into());
    }

    // Create directory structure
    fs::create_dir_all(&git_dir)?;
    fs::create_dir_all(git_dir.join("objects"))?;
    fs::create_dir_all(git_dir.join("refs").join("heads"))?;
    fs::create_dir_all(git_dir.join("refs").join("remotes"))?;

    // Create HEAD file pointing to main branch
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main")?;

    // Create empty config file
    fs::write(
        git_dir.join("config"),
        "[core]\n\trepositoryformatversion = 0\n",
    )?;

    println!(
        "Initialized empty Mini Git repository in {}",
        git_dir.display()
    );
    Ok(())
}
