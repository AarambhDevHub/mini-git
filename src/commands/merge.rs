use crate::{Commit, Repository, Result, Tree, TreeEntry, object_store::ObjectStore, utils};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;

pub fn merge(repo: &Repository, branch_name: String, author: Option<String>) -> Result<()> {
    let current_branch = utils::get_current_branch(repo)?;
    if current_branch == branch_name {
        return Err("Cannot merge branch into itself".into());
    }

    let current_commit =
        utils::get_branch_commit(repo, &current_branch)?.ok_or("Current branch has no commits")?;

    let merge_commit = utils::get_branch_commit(repo, &branch_name)?
        .ok_or(format!("Branch '{}' not found", branch_name))?;

    if current_commit == merge_commit {
        println!("Already up to date.");
        return Ok(());
    }

    let object_store = ObjectStore::new(repo);

    // Check if it's a fast-forward merge
    if is_ancestor(&object_store, &current_commit, &merge_commit)? {
        // Fast-forward merge
        utils::update_branch(repo, &current_branch, &merge_commit)?;
        crate::commands::checkout(repo, current_branch)?;
        println!("Fast-forward merge completed");
        return Ok(());
    }

    // Three-way merge
    let common_ancestor = find_common_ancestor(&object_store, &current_commit, &merge_commit)?
        .ok_or("No common ancestor found")?;

    println!("Performing three-way merge...");
    println!("Base: {}", &common_ancestor[..8]);
    println!("Ours: {}", &current_commit[..8]);
    println!("Theirs: {}", &merge_commit[..8]);

    let merged_tree = perform_three_way_merge(
        &object_store,
        &common_ancestor,
        &current_commit,
        &merge_commit,
    )?;

    // Create merge commit
    let author = author.unwrap_or_else(|| "Mini Git <minigit@example.com>".to_string());
    let message = format!("Merge branch '{}' into {}", branch_name, current_branch);
    let commit_content = format!(
        "{}{}{}{}{}",
        merged_tree.hash, current_commit, merge_commit, author, message
    );
    let commit_hash = ObjectStore::hash_content(commit_content.as_bytes());

    let merge_commit_obj = Commit {
        hash: commit_hash.clone(),
        parent: Some(current_commit),
        tree: merged_tree.hash.clone(),
        author,
        message,
        timestamp: Utc::now(),
    };

    object_store.store_commit(&merge_commit_obj)?;
    utils::update_branch(repo, &current_branch, &commit_hash)?;

    // Update working directory
    restore_tree_to_working_dir(repo, &object_store, &merged_tree)?;

    println!("Merge completed: {}", &commit_hash[..8]);
    Ok(())
}

fn is_ancestor(object_store: &ObjectStore, ancestor: &str, descendant: &str) -> Result<bool> {
    let mut current = descendant.to_string();

    while current != ancestor {
        let commit = object_store.load_commit(&current)?;
        if let Some(parent) = commit.parent {
            current = parent;
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

fn find_common_ancestor(
    object_store: &ObjectStore,
    commit1: &str,
    commit2: &str,
) -> Result<Option<String>> {
    let mut ancestors1 = std::collections::HashSet::new();
    let mut current = commit1.to_string();

    // Collect all ancestors of commit1
    loop {
        ancestors1.insert(current.clone());
        let commit = object_store.load_commit(&current)?;
        if let Some(parent) = commit.parent {
            current = parent;
        } else {
            break;
        }
    }

    // Find first common ancestor in commit2's history
    current = commit2.to_string();
    loop {
        if ancestors1.contains(&current) {
            return Ok(Some(current));
        }

        let commit = object_store.load_commit(&current)?;
        if let Some(parent) = commit.parent {
            current = parent;
        } else {
            break;
        }
    }

    Ok(None)
}

fn perform_three_way_merge(
    object_store: &ObjectStore,
    base_commit: &str,
    our_commit: &str,
    their_commit: &str,
) -> Result<Tree> {
    let base_tree = {
        let commit = object_store.load_commit(base_commit)?;
        object_store.load_tree(&commit.tree)?
    };

    let our_tree = {
        let commit = object_store.load_commit(our_commit)?;
        object_store.load_tree(&commit.tree)?
    };

    let their_tree = {
        let commit = object_store.load_commit(their_commit)?;
        object_store.load_tree(&commit.tree)?
    };

    let mut merged_entries = HashMap::new();
    let mut all_paths = std::collections::HashSet::new();

    // Collect all file paths
    for path in base_tree.entries.keys() {
        all_paths.insert(path.clone());
    }
    for path in our_tree.entries.keys() {
        all_paths.insert(path.clone());
    }
    for path in their_tree.entries.keys() {
        all_paths.insert(path.clone());
    }

    // Merge each file
    for path in all_paths {
        let base_entry = base_tree.entries.get(&path);
        let our_entry = our_tree.entries.get(&path);
        let their_entry = their_tree.entries.get(&path);

        match (base_entry, our_entry, their_entry) {
            // File unchanged in both branches
            (Some(base), Some(our), Some(their))
                if our.hash == base.hash && their.hash == base.hash =>
            {
                merged_entries.insert(path, base.clone());
            }
            // File changed only in our branch
            (Some(_), Some(our), Some(their)) if their.hash == base_entry.unwrap().hash => {
                merged_entries.insert(path, our.clone());
            }
            // File changed only in their branch
            (Some(_), Some(our), Some(their)) if our.hash == base_entry.unwrap().hash => {
                merged_entries.insert(path, their.clone());
            }
            // File added in our branch only
            (None, Some(our), None) => {
                merged_entries.insert(path, our.clone());
            }
            // File added in their branch only
            (None, None, Some(their)) => {
                merged_entries.insert(path, their.clone());
            }
            // File deleted in our branch
            (Some(_), None, Some(their)) if their.hash == base_entry.unwrap().hash => {
                // Keep deleted (don't add to merged_entries)
            }
            // File deleted in their branch
            (Some(_), Some(our), None) if our.hash == base_entry.unwrap().hash => {
                // Keep deleted (don't add to merged_entries)
            }
            // Conflict: both branches modified the file differently
            (Some(_), Some(our), Some(their)) if our.hash != their.hash => {
                println!("CONFLICT: Merge conflict in {}", path);
                println!("Automatic merge failed; using our version");
                merged_entries.insert(path, our.clone());
            }
            // Other cases: use default behavior
            _ => {
                if let Some(entry) = our_entry.or(their_entry) {
                    merged_entries.insert(path, entry.clone());
                }
            }
        }
    }

    // Create merged tree
    let tree_content = serde_json::to_vec(&merged_entries)?;
    let tree_hash = ObjectStore::hash_content(&tree_content);
    let merged_tree = Tree {
        hash: tree_hash,
        entries: merged_entries,
    };

    object_store.store_tree(&merged_tree)?;
    Ok(merged_tree)
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
