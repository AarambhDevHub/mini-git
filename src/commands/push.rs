use crate::{Repository, Result, utils};
use std::fs;
use std::path::PathBuf;

pub fn push(repo: &Repository, remote: Option<String>, branch: Option<String>) -> Result<()> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());
    let branch_name =
        branch.unwrap_or_else(|| utils::get_current_branch(repo).unwrap_or("main".to_string()));

    // Get remote URL from config
    let remote_url = get_remote_url(repo, &remote_name)?;

    println!("Pushing to {} ({})", remote_name, remote_url);

    // Get current branch commit
    let local_commit = utils::get_branch_commit(repo, &branch_name)?
        .ok_or(format!("Branch '{}' has no commits", branch_name))?;

    // Only handle local file path remotes
    if PathBuf::from(&remote_url).exists() {
        push_to_local_remote(repo, &remote_url, &branch_name, &local_commit)?;
    } else {
        println!("Note: Mini Git only supports local repository pushing.");
        println!("Remote URL: {}", remote_url);
        println!(
            "Local commit {} is ready to be pushed to {}/{}",
            &local_commit[..8],
            remote_name,
            branch_name
        );
        println!(
            "For network remotes, use standard Git: git push {} {}",
            remote_name, branch_name
        );

        // Update local remote tracking branch for consistency
        let remote_branch_path = repo
            .git_dir
            .join("refs")
            .join("remotes")
            .join(&remote_name)
            .join(&branch_name);
        fs::create_dir_all(remote_branch_path.parent().unwrap())?;
        fs::write(remote_branch_path, &local_commit)?;

        println!(
            "Updated local tracking branch {}/{}",
            remote_name, branch_name
        );
    }

    Ok(())
}

fn push_to_local_remote(
    repo: &Repository,
    remote_path: &str,
    branch_name: &str,
    commit_hash: &str,
) -> Result<()> {
    let remote_git_dir = PathBuf::from(remote_path).join(".mini_git");
    if !remote_git_dir.exists() {
        return Err("Remote is not a mini-git repository".into());
    }

    println!("Pushing to local Mini Git repository...");

    // Copy objects that don't exist in remote
    let local_objects = repo.git_dir.join("objects");
    let remote_objects = remote_git_dir.join("objects");

    let copied_count = copy_missing_objects(&local_objects, &remote_objects)?;

    // Create remote repository struct
    let remote_repo = Repository {
        git_dir: remote_git_dir.clone(),
        work_dir: PathBuf::from(remote_path).to_path_buf(),
    };

    // Check if remote has uncommitted changes
    let remote_has_changes = check_for_uncommitted_changes(&remote_repo)?;

    // Update remote branch
    let remote_branch_path = remote_git_dir.join("refs").join("heads").join(branch_name);
    fs::create_dir_all(remote_branch_path.parent().unwrap())?;

    let old_commit = if remote_branch_path.exists() {
        Some(fs::read_to_string(&remote_branch_path)?.trim().to_string())
    } else {
        None
    };

    fs::write(remote_branch_path, commit_hash)?;

    // Update remote working directory if safe to do so
    if !remote_has_changes {
        println!("Updating remote working directory...");
        update_remote_working_directory(&remote_repo, commit_hash)?;
        println!("Remote working directory updated with new files");
    } else {
        println!("Warning: Remote repository has uncommitted changes.");
        println!(
            "Working directory not updated. Run './mini_git checkout {}' in remote repository.",
            branch_name
        );
    }

    // Update local remote tracking branch
    let local_remote_branch_path = repo
        .git_dir
        .join("refs")
        .join("remotes")
        .join("origin")
        .join(branch_name);
    fs::create_dir_all(local_remote_branch_path.parent().unwrap())?;
    fs::write(local_remote_branch_path, commit_hash)?;

    println!(
        "Successfully pushed {} to origin/{}",
        &commit_hash[..8],
        branch_name
    );
    println!("Copied {} objects to remote repository", copied_count);

    if let Some(old) = old_commit {
        if old != commit_hash {
            println!(
                "Updated remote branch from {} to {}",
                &old[..8],
                &commit_hash[..8]
            );
        }
    }

    Ok(())
}

fn check_for_uncommitted_changes(repo: &Repository) -> Result<bool> {
    let index = utils::load_index(repo)?;

    // Check if working directory matches index
    for (path, index_entry) in &index.entries {
        let file_path = repo.work_dir.join(path);
        if file_path.exists() {
            let content = fs::read(&file_path)?;
            let current_hash = crate::object_store::ObjectStore::hash_content(&content);
            if current_hash != index_entry.hash {
                return Ok(true); // Modified file
            }
        } else {
            return Ok(true); // Deleted file
        }
    }

    // Check for untracked files
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

        if !index.entries.contains_key(&relative_path) {
            return Ok(true); // Untracked file
        }
    }

    Ok(false)
}

fn update_remote_working_directory(repo: &Repository, commit_hash: &str) -> Result<()> {
    let object_store = crate::object_store::ObjectStore::new(repo);
    let commit = object_store.load_commit(commit_hash)?;
    let tree = object_store.load_tree(&commit.tree)?;

    // Clear existing files (except .mini_git)
    for entry in fs::read_dir(&repo.work_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == ".mini_git" {
            continue;
        }

        if path.is_file() {
            let _ = fs::remove_file(&path); // Ignore errors for now
        } else if path.is_dir() {
            let _ = fs::remove_dir_all(&path); // Ignore errors for now
        }
    }

    // Restore files from tree
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

    // Update index to match
    let mut new_index = crate::Index {
        entries: std::collections::HashMap::new(),
    };

    for (path, tree_entry) in &tree.entries {
        if tree_entry.is_file {
            new_index.entries.insert(
                path.clone(),
                crate::IndexEntry {
                    hash: tree_entry.hash.clone(),
                    mode: tree_entry.mode.clone(),
                    path: path.clone(),
                },
            );
        }
    }

    utils::save_index(repo, &new_index)?;
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
