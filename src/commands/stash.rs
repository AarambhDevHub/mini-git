use crate::{
    Commit, Index, IndexEntry, Repository, Result, Tree, TreeEntry, object_store::ObjectStore,
    utils,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stash {
    message: String,
    commit_hash: String,
    parent_commit: Option<String>,
    index_tree: String,
    working_tree: String,
    timestamp: chrono::DateTime<Utc>,
}

pub fn stash(
    repo: &Repository,
    action: Option<String>,
    message: Option<String>,
    index: Option<usize>,
) -> Result<()> {
    match action.as_deref() {
        Some("push") | None => {
            stash_push(repo, message)?;
        }
        Some("pop") => {
            stash_pop(repo, index)?;
        }
        Some("list") => {
            stash_list(repo)?;
        }
        Some("show") => {
            stash_show(repo, index)?;
        }
        Some("drop") => {
            stash_drop(repo, index)?;
        }
        Some("clear") => {
            stash_clear(repo)?;
        }
        _ => {
            return Err("Invalid stash action. Use: push, pop, list, show, drop, clear".into());
        }
    }

    Ok(())
}

fn stash_push(repo: &Repository, message: Option<String>) -> Result<()> {
    let index = utils::load_index(repo)?;
    let object_store = ObjectStore::new(repo);

    // Check if there are any changes to stash
    if index.entries.is_empty() && !has_unstaged_changes(repo)? {
        println!("No local changes to save");
        return Ok(());
    }

    // Create stash entry
    let message = message.unwrap_or_else(|| {
        format!(
            "WIP on {}: {}",
            utils::get_current_branch(repo).unwrap_or("HEAD".to_string()),
            get_last_commit_subject(repo).unwrap_or("unknown".to_string())
        )
    });

    // Save current state
    let index_tree = create_tree_from_index(repo, &object_store, &index)?;
    let working_tree = create_tree_from_working_dir(repo, &object_store)?;

    let current_branch = utils::get_current_branch(repo)?;
    let parent_commit = utils::get_branch_commit(repo, &current_branch)?;

    // Create stash commit
    let stash_content = format!(
        "{}{}{}{}",
        working_tree.hash,
        parent_commit.as_ref().unwrap_or(&String::new()),
        "Mini Git Stash <stash@minigit.local>",
        message
    );
    let stash_hash = ObjectStore::hash_content(stash_content.as_bytes());

    let stash_commit = Commit {
        hash: stash_hash.clone(),
        parent: parent_commit.clone(),
        tree: working_tree.hash.clone(),
        author: "Mini Git Stash <stash@minigit.local>".to_string(),
        message: message.clone(),
        timestamp: Utc::now(),
    };

    object_store.store_commit(&stash_commit)?;

    // Save stash entry
    let stash_entry = Stash {
        message,
        commit_hash: stash_hash,
        parent_commit,
        index_tree: index_tree.hash,
        working_tree: working_tree.hash,
        timestamp: Utc::now(),
    };

    save_stash_entry(repo, &stash_entry)?;

    // Clean working directory and index
    clear_working_directory(repo)?;
    let empty_index = Index {
        entries: HashMap::new(),
    };
    utils::save_index(repo, &empty_index)?;

    println!("Saved working directory and index state");
    Ok(())
}

fn stash_pop(repo: &Repository, index: Option<usize>) -> Result<()> {
    let stash_entries = load_stash_entries(repo)?;
    let stash_index = index.unwrap_or(0);

    if stash_index >= stash_entries.len() {
        return Err("Invalid stash index".into());
    }

    let stash_entry = &stash_entries[stash_index];
    let object_store = ObjectStore::new(repo);

    // Restore working directory from stash
    let working_tree = object_store.load_tree(&stash_entry.working_tree)?;
    restore_tree_to_working_dir(repo, &object_store, &working_tree)?;

    // Restore index from stash
    let index_tree = object_store.load_tree(&stash_entry.index_tree)?;
    let restored_index = create_index_from_tree(&index_tree);
    utils::save_index(repo, &restored_index)?;

    // Remove stash entry
    let mut remaining_stashes = stash_entries.clone();
    remaining_stashes.remove(stash_index);
    save_stash_entries(repo, &remaining_stashes)?;

    println!("Applied stash@{{{}}}: {}", stash_index, stash_entry.message);
    println!("Dropped stash@{{{}}}", stash_index);

    Ok(())
}

fn stash_list(repo: &Repository) -> Result<()> {
    let stash_entries = load_stash_entries(repo)?;

    if stash_entries.is_empty() {
        println!("No stashes found");
        return Ok(());
    }

    for (i, stash) in stash_entries.iter().enumerate() {
        println!("stash@{{{}}}: {}", i, stash.message);
    }

    Ok(())
}

fn stash_show(repo: &Repository, index: Option<usize>) -> Result<()> {
    let stash_entries = load_stash_entries(repo)?;
    let stash_index = index.unwrap_or(0);

    if stash_index >= stash_entries.len() {
        return Err("Invalid stash index".into());
    }

    let stash_entry = &stash_entries[stash_index];

    println!("stash@{{{}}}: {}", stash_index, stash_entry.message);
    println!(
        "Date: {}",
        stash_entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Commit: {}", stash_entry.commit_hash);

    // Show diff (simplified)
    let object_store = ObjectStore::new(repo);
    let working_tree = object_store.load_tree(&stash_entry.working_tree)?;

    println!("\nFiles in stash:");
    for (path, _) in &working_tree.entries {
        println!("  {}", path);
    }

    Ok(())
}

fn stash_drop(repo: &Repository, index: Option<usize>) -> Result<()> {
    let mut stash_entries = load_stash_entries(repo)?;
    let stash_index = index.unwrap_or(0);

    if stash_index >= stash_entries.len() {
        return Err("Invalid stash index".into());
    }

    let dropped_stash = stash_entries.remove(stash_index);
    save_stash_entries(repo, &stash_entries)?;

    println!(
        "Dropped stash@{{{}}}: {}",
        stash_index, dropped_stash.message
    );
    Ok(())
}

fn stash_clear(repo: &Repository) -> Result<()> {
    let stash_path = repo.git_dir.join("stash");
    if stash_path.exists() {
        fs::remove_file(stash_path)?;
    }

    println!("Cleared all stashes");
    Ok(())
}

fn has_unstaged_changes(repo: &Repository) -> Result<bool> {
    let index = utils::load_index(repo)?;

    // Check if any tracked files have been modified
    for (path, index_entry) in &index.entries {
        let file_path = repo.work_dir.join(path);
        if file_path.exists() {
            let content = fs::read(&file_path)?;
            let current_hash = ObjectStore::hash_content(&content);
            if current_hash != index_entry.hash {
                return Ok(true);
            }
        } else {
            // File was deleted
            return Ok(true);
        }
    }

    Ok(false)
}

fn get_last_commit_subject(repo: &Repository) -> Result<String> {
    let current_branch = utils::get_current_branch(repo)?;
    if let Some(commit_hash) = utils::get_branch_commit(repo, &current_branch)? {
        let object_store = ObjectStore::new(repo);
        let commit = object_store.load_commit(&commit_hash)?;

        // Get first line of commit message
        let first_line = commit.message.lines().next().unwrap_or("");
        Ok(first_line.to_string())
    } else {
        Ok("no commits yet".to_string())
    }
}

fn create_tree_from_index(
    repo: &Repository,
    object_store: &ObjectStore,
    index: &Index,
) -> Result<Tree> {
    let mut tree_entries = HashMap::new();

    for (path, index_entry) in &index.entries {
        tree_entries.insert(
            path.clone(),
            TreeEntry {
                mode: index_entry.mode.clone(),
                hash: index_entry.hash.clone(),
                name: path.clone(),
                is_file: true,
            },
        );
    }

    let tree_content = serde_json::to_vec(&tree_entries)?;
    let tree_hash = ObjectStore::hash_content(&tree_content);
    let tree = Tree {
        hash: tree_hash,
        entries: tree_entries,
    };

    object_store.store_tree(&tree)?;
    Ok(tree)
}

fn create_tree_from_working_dir(repo: &Repository, object_store: &ObjectStore) -> Result<Tree> {
    let mut tree_entries = HashMap::new();

    for entry in walkdir::WalkDir::new(&repo.work_dir)
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

        let content = fs::read(path)?;
        let hash = object_store.store_blob(&content)?;

        tree_entries.insert(
            relative_path.clone(),
            TreeEntry {
                mode: "100644".to_string(),
                hash,
                name: relative_path,
                is_file: true,
            },
        );
    }

    let tree_content = serde_json::to_vec(&tree_entries)?;
    let tree_hash = ObjectStore::hash_content(&tree_content);
    let tree = Tree {
        hash: tree_hash,
        entries: tree_entries,
    };

    object_store.store_tree(&tree)?;
    Ok(tree)
}

fn create_index_from_tree(tree: &Tree) -> Index {
    let mut entries = HashMap::new();

    for (path, tree_entry) in &tree.entries {
        if tree_entry.is_file {
            entries.insert(
                path.clone(),
                IndexEntry {
                    hash: tree_entry.hash.clone(),
                    mode: tree_entry.mode.clone(),
                    path: path.clone(),
                },
            );
        }
    }

    Index { entries }
}

fn restore_tree_to_working_dir(
    repo: &Repository,
    object_store: &ObjectStore,
    tree: &Tree,
) -> Result<()> {
    for (path, tree_entry) in &tree.entries {
        if tree_entry.is_file {
            let blob = object_store.load_blob(&tree_entry.hash)?;
            let file_path = repo.work_dir.join(path);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(file_path, &blob.content)?;
        }
    }

    Ok(())
}

fn clear_working_directory(repo: &Repository) -> Result<()> {
    for entry in fs::read_dir(&repo.work_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == ".mini_git" {
            continue;
        }

        if path.is_file() {
            fs::remove_file(&path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

fn load_stash_entries(repo: &Repository) -> Result<Vec<Stash>> {
    let stash_path = repo.git_dir.join("stash");
    if stash_path.exists() {
        let content = fs::read_to_string(stash_path)?;
        let entries: Vec<Stash> = serde_json::from_str(&content)?;
        Ok(entries)
    } else {
        Ok(Vec::new())
    }
}

fn save_stash_entries(repo: &Repository, entries: &[Stash]) -> Result<()> {
    let stash_path = repo.git_dir.join("stash");
    let content = serde_json::to_string_pretty(entries)?;
    fs::write(stash_path, content)?;
    Ok(())
}

fn save_stash_entry(repo: &Repository, stash: &Stash) -> Result<()> {
    let mut entries = load_stash_entries(repo)?;
    entries.insert(0, stash.clone()); // Insert at beginning (most recent first)
    save_stash_entries(repo, &entries)?;
    Ok(())
}
