use crate::{Repository, Result, utils};
use std::collections::HashSet;
use std::fs;
use walkdir::WalkDir;

pub fn status(repo: &Repository) -> Result<()> {
    let current_branch = utils::get_current_branch(repo)?;
    println!("On branch {}", current_branch);

    let index = utils::load_index(repo)?;

    // Get all files in working directory
    let mut working_files = HashSet::new();
    for entry in WalkDir::new(&repo.work_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.starts_with(&repo.git_dir) {
            continue;
        }

        let relative_path = path
            .strip_prefix(&repo.work_dir)?
            .to_string_lossy()
            .replace('\\', "/");
        working_files.insert(relative_path);
    }

    // Check staged files
    let mut staged_files = Vec::new();
    let mut modified_files = Vec::new();

    for (path, entry) in &index.entries {
        staged_files.push(path.clone());

        // Check if file is modified
        let file_path = repo.work_dir.join(path);
        if file_path.exists() {
            let content = fs::read(&file_path)?;
            let current_hash = crate::object_store::ObjectStore::hash_content(&content);
            if current_hash != entry.hash {
                modified_files.push(path.clone());
            }
        }

        working_files.remove(path);
    }

    // Print status
    if !staged_files.is_empty() {
        println!("\nChanges to be committed:");
        for file in &staged_files {
            println!("  new file:   {}", file);
        }
    }

    if !modified_files.is_empty() {
        println!("\nChanges not staged for commit:");
        for file in &modified_files {
            println!("  modified:   {}", file);
        }
    }

    if !working_files.is_empty() {
        println!("\nUntracked files:");
        for file in &working_files {
            println!("  {}", file);
        }
    }

    if staged_files.is_empty() && modified_files.is_empty() && working_files.is_empty() {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
}
