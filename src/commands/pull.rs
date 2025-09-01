use crate::{Repository, Result, utils};
use std::fs;
use std::path::PathBuf;

pub fn pull(repo: &Repository, remote: Option<String>, branch: Option<String>) -> Result<()> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());
    let branch_name =
        branch.unwrap_or_else(|| utils::get_current_branch(repo).unwrap_or("main".to_string()));

    println!("Pulling from {} {}", remote_name, branch_name);

    // First, fetch from remote
    fetch(repo, Some(remote_name.clone()), Some(branch_name.clone()))?;

    // Then merge the remote branch
    let remote_commit = get_remote_branch_commit(repo, &remote_name, &branch_name)?;

    if let Some(remote_commit_hash) = remote_commit {
        let current_commit = utils::get_branch_commit(repo, &branch_name)?;

        if let Some(current_hash) = current_commit {
            if current_hash == remote_commit_hash {
                println!("Already up to date.");
                return Ok(());
            }

            // Check if it's a fast-forward merge
            let object_store = crate::object_store::ObjectStore::new(repo);
            if is_ancestor(&object_store, &current_hash, &remote_commit_hash)? {
                // Fast-forward merge
                utils::update_branch(repo, &branch_name, &remote_commit_hash)?;
                crate::commands::checkout(repo, branch_name)?;
                println!("Fast-forward to {}", &remote_commit_hash[..8]);
            } else {
                println!("Note: Non-fast-forward merge requires manual merge command");
                println!("Run: mini_git merge {}/{}", remote_name, branch_name);
            }
        } else {
            // No local commits, just fast-forward
            utils::update_branch(repo, &branch_name, &remote_commit_hash)?;
            crate::commands::checkout(repo, branch_name)?;
            println!("Fast-forward to {}", &remote_commit_hash[..8]);
        }
    } else {
        println!("No commits found in remote branch");
    }

    Ok(())
}

pub fn fetch(repo: &Repository, remote: Option<String>, branch: Option<String>) -> Result<()> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());
    let branch_name = branch.unwrap_or_else(|| "main".to_string());

    let remote_url = get_remote_url(repo, &remote_name)?;
    println!("Fetching from {} ({})", remote_name, remote_url);

    // Only handle local file path remotes
    if PathBuf::from(&remote_url).exists() {
        fetch_from_local_remote(repo, &remote_url, &remote_name, &branch_name)?;
    } else {
        println!("Note: Mini Git only supports local repository fetching.");
        println!("Remote URL: {}", remote_url);
        println!(
            "For network remotes, use standard Git: git fetch {} {}",
            remote_name, branch_name
        );
    }

    Ok(())
}

fn fetch_from_local_remote(
    repo: &Repository,
    remote_path: &str,
    remote_name: &str,
    branch_name: &str,
) -> Result<()> {
    let remote_git_dir = PathBuf::from(remote_path).join(".mini_git");
    if !remote_git_dir.exists() {
        return Err("Remote is not a mini-git repository".into());
    }

    println!("Fetching from local Mini Git repository...");

    // Copy missing objects from remote
    let remote_objects = remote_git_dir.join("objects");
    let local_objects = repo.git_dir.join("objects");

    let copied_count = copy_missing_objects(&remote_objects, &local_objects)?;

    // Update remote tracking branch
    let remote_branch_path = remote_git_dir.join("refs").join("heads").join(branch_name);
    if remote_branch_path.exists() {
        let remote_commit = fs::read_to_string(remote_branch_path)?.trim().to_string();

        let local_remote_branch_path = repo
            .git_dir
            .join("refs")
            .join("remotes")
            .join(remote_name)
            .join(branch_name);

        fs::create_dir_all(local_remote_branch_path.parent().unwrap())?;
        fs::write(local_remote_branch_path, &remote_commit)?;

        println!(
            "Updated {}/{} to {}",
            remote_name,
            branch_name,
            &remote_commit[..8]
        );
        println!("Fetched {} objects from remote repository", copied_count);
    }

    Ok(())
}

fn copy_missing_objects(src_objects: &PathBuf, dst_objects: &PathBuf) -> Result<usize> {
    if !src_objects.exists() {
        return Ok(0);
    }

    fs::create_dir_all(dst_objects)?;
    let mut copied_count = 0;

    for entry in fs::read_dir(src_objects)? {
        let entry = entry?;
        let src_dir = entry.path();

        if src_dir.is_dir() {
            let dir_name = entry.file_name();
            let dst_dir = dst_objects.join(&dir_name);
            fs::create_dir_all(&dst_dir)?;

            for obj_entry in fs::read_dir(&src_dir)? {
                let obj_entry = obj_entry?;
                let src_obj = obj_entry.path();
                let dst_obj = dst_dir.join(obj_entry.file_name());

                if !dst_obj.exists() {
                    fs::copy(&src_obj, &dst_obj)?;
                    copied_count += 1;
                }
            }
        }
    }

    Ok(copied_count)
}

fn get_remote_url(repo: &Repository, remote_name: &str) -> Result<String> {
    let config_path = repo.git_dir.join("config");
    let config_content = fs::read_to_string(config_path)?;

    let lines: Vec<&str> = config_content.lines().collect();
    let mut in_remote_section = false;
    let remote_header = format!("[remote \"{}\"]", remote_name);

    for line in lines {
        let line = line.trim();
        if line == remote_header {
            in_remote_section = true;
            continue;
        }

        if in_remote_section {
            if line.starts_with('[') && line.ends_with(']') {
                break;
            }

            if line.starts_with("url = ") {
                return Ok(line.replace("url = ", ""));
            }
        }
    }

    Err(format!("Remote '{}' not found", remote_name).into())
}

fn get_remote_branch_commit(
    repo: &Repository,
    remote_name: &str,
    branch_name: &str,
) -> Result<Option<String>> {
    let remote_branch_path = repo
        .git_dir
        .join("refs")
        .join("remotes")
        .join(remote_name)
        .join(branch_name);

    if remote_branch_path.exists() {
        let commit = fs::read_to_string(remote_branch_path)?.trim().to_string();
        Ok(Some(commit))
    } else {
        Ok(None)
    }
}

fn is_ancestor(
    object_store: &crate::object_store::ObjectStore,
    ancestor: &str,
    descendant: &str,
) -> Result<bool> {
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
