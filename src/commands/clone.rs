use crate::{Repository, Result, object_store::ObjectStore, utils};
use std::fs;
use std::path::PathBuf;

pub fn clone(url: String, directory: Option<String>) -> Result<()> {
    let source_path = PathBuf::from(&url);

    // Check if source is a local path
    if !source_path.exists() {
        println!("Note: Mini Git only supports cloning from local repositories.");
        println!("Source path '{}' does not exist.", url);
        println!("For network remotes, use standard Git: git clone {}", url);
        return Ok(());
    }

    let dir_name = directory.unwrap_or_else(|| {
        source_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let target_dir = PathBuf::from(&dir_name);
    if target_dir.exists() {
        return Err(format!("Directory '{}' already exists", dir_name).into());
    }

    println!(
        "Cloning local repository from '{}' into '{}'...",
        url, dir_name
    );

    // Create target directory
    fs::create_dir_all(&target_dir)?;

    // Initialize repository in target directory
    let git_dir = target_dir.join(".mini_git");
    fs::create_dir_all(&git_dir)?;
    fs::create_dir_all(git_dir.join("objects"))?;
    fs::create_dir_all(git_dir.join("refs").join("heads"))?;
    fs::create_dir_all(git_dir.join("refs").join("remotes").join("origin"))?;

    // Set up repository structure
    let repo = Repository {
        git_dir: git_dir.clone(),
        work_dir: target_dir.clone(),
    };

    // Add remote origin
    add_remote(&repo, "origin".to_string(), url.clone())?;

    // Create HEAD pointing to main
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main")?;

    // Clone from local repository
    clone_local(&repo, &url)?;

    println!("Clone completed successfully");
    Ok(())
}

fn clone_local(repo: &Repository, source_path: &str) -> Result<()> {
    let source_git_dir = PathBuf::from(source_path).join(".mini_git");
    if !source_git_dir.exists() {
        return Err("Source is not a mini-git repository".into());
    }

    println!("Copying repository data...");

    // Copy objects
    let source_objects = source_git_dir.join("objects");
    let target_objects = repo.git_dir.join("objects");

    let mut objects_copied = 0;
    if source_objects.exists() {
        objects_copied = copy_dir_recursive(&source_objects, &target_objects)?;
    }

    // Copy refs
    let source_refs = source_git_dir.join("refs");
    let target_refs = repo.git_dir.join("refs");

    if source_refs.exists() {
        copy_refs(&source_refs, &target_refs)?;
    }

    // Get the main branch commit and checkout
    if let Some(main_commit) = utils::get_branch_commit(repo, "main")? {
        checkout_commit(repo, &main_commit)?;
        println!("Checked out main branch at commit {}", &main_commit[..8]);
    } else {
        println!("No commits found in source repository");
    }

    println!("Copied {} objects from source repository", objects_copied);
    Ok(())
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<usize> {
    fs::create_dir_all(dst)?;
    let mut files_copied = 0;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            files_copied += copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
            files_copied += 1;
        }
    }

    Ok(files_copied)
}

fn copy_refs(src_refs: &PathBuf, dst_refs: &PathBuf) -> Result<()> {
    // Copy heads (branches)
    let src_heads = src_refs.join("heads");
    let dst_heads = dst_refs.join("heads");

    if src_heads.exists() {
        fs::create_dir_all(&dst_heads)?;
        for entry in fs::read_dir(&src_heads)? {
            let entry = entry?;
            let branch_name = entry.file_name();
            let src_file = src_heads.join(&branch_name);
            let dst_file = dst_heads.join(&branch_name);

            if src_file.is_file() {
                fs::copy(&src_file, &dst_file)?;

                // Also create remote tracking branch
                let remote_branch_dir = dst_refs.join("remotes").join("origin");
                fs::create_dir_all(&remote_branch_dir)?;
                let remote_branch_file = remote_branch_dir.join(&branch_name);
                fs::copy(&src_file, &remote_branch_file)?;
            }
        }
    }

    Ok(())
}

fn checkout_commit(repo: &Repository, commit_hash: &str) -> Result<()> {
    let object_store = ObjectStore::new(repo);
    let commit = object_store.load_commit(commit_hash)?;
    let tree = object_store.load_tree(&commit.tree)?;

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

    Ok(())
}

pub fn add_remote(repo: &Repository, name: String, url: String) -> Result<()> {
    let config_path = repo.git_dir.join("config");
    let mut config_content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        "[core]\n\trepositoryformatversion = 0\n".to_string()
    };

    // Add remote configuration
    config_content.push_str(&format!(
        "\n[remote \"{}\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/{}/*\n",
        name, url, name
    ));

    fs::write(config_path, config_content)?;

    Ok(())
}
