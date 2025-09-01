use crate::{Repository, Result};
use std::fs;
use std::path::PathBuf;

pub fn remote(
    repo: &Repository,
    action: Option<String>,
    name: Option<String>,
    url: Option<String>,
) -> Result<()> {
    match action.as_deref() {
        Some("add") => {
            let name = name.ok_or("Remote name required")?;
            let url = url.ok_or("Remote URL required")?;
            add_remote(repo, name, url)?;
        }
        Some("remove") | Some("rm") => {
            let name = name.ok_or("Remote name required")?;
            remove_remote(repo, name)?;
        }
        Some("set-url") => {
            let name = name.ok_or("Remote name required")?;
            let url = url.ok_or("Remote URL required")?;
            set_remote_url(repo, name, url)?;
        }
        Some("get-url") => {
            let name = name.ok_or("Remote name required")?;
            get_remote_url(repo, name)?;
        }
        Some("-v") | Some("--verbose") | None => {
            list_remotes(repo, action.is_some())?;
        }
        _ => {
            return Err("Invalid remote action. Use: add, remove, set-url, get-url, or -v".into());
        }
    }

    Ok(())
}

fn add_remote(repo: &Repository, name: String, url: String) -> Result<()> {
    // Validate URL for local-only approach
    if !is_local_path(&url) {
        println!("Note: Mini Git only supports local repository remotes.");
        println!("Adding remote '{}' -> {} (for reference only)", name, url);
        println!("Push/pull operations will only work with local file paths.");
    } else {
        // Validate that the local path exists and is a mini-git repository
        let path = PathBuf::from(&url);
        if !path.exists() {
            return Err(format!("Local path '{}' does not exist", url).into());
        }

        let mini_git_dir = path.join(".mini_git");
        if !mini_git_dir.exists() {
            return Err(format!("'{}' is not a Mini Git repository", url).into());
        }

        println!("Adding local Mini Git remote: '{}' -> {}", name, url);
    }

    let config_path = repo.git_dir.join("config");
    let mut config_content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        "[core]\n\trepositoryformatversion = 0\n".to_string()
    };

    // Check if remote already exists
    if config_content.contains(&format!("[remote \"{}\"]", name)) {
        return Err(format!("Remote '{}' already exists", name).into());
    }

    // Add remote configuration
    config_content.push_str(&format!(
        "\n[remote \"{}\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/{}/*\n",
        name, url, name
    ));

    fs::write(config_path, config_content)?;

    // Create remote refs directory
    let remote_refs_dir = repo.git_dir.join("refs").join("remotes").join(&name);
    fs::create_dir_all(remote_refs_dir)?;

    Ok(())
}

fn remove_remote(repo: &Repository, name: String) -> Result<()> {
    let config_path = repo.git_dir.join("config");
    let config_content = fs::read_to_string(&config_path)?;

    let lines: Vec<&str> = config_content.lines().collect();
    let mut new_lines = Vec::new();
    let mut skip_section = false;
    let remote_header = format!("[remote \"{}\"]", name);
    let mut found = false;

    for line in lines {
        let line_trimmed = line.trim();

        if line_trimmed == remote_header {
            skip_section = true;
            found = true;
            continue;
        }

        if skip_section {
            if line_trimmed.starts_with('[') && line_trimmed.ends_with(']') {
                // Entered a new section
                skip_section = false;
                new_lines.push(line);
            }
            // Skip lines in the remote section
        } else {
            new_lines.push(line);
        }
    }

    if !found {
        return Err(format!("Remote '{}' does not exist", name).into());
    }

    fs::write(config_path, new_lines.join("\n"))?;

    // Remove remote refs directory
    let remote_refs_dir = repo.git_dir.join("refs").join("remotes").join(&name);
    if remote_refs_dir.exists() {
        fs::remove_dir_all(remote_refs_dir)?;
    }

    println!("Removed remote '{}'", name);
    Ok(())
}

fn set_remote_url(repo: &Repository, name: String, new_url: String) -> Result<()> {
    // Validate URL for local-only approach
    if !is_local_path(&new_url) {
        println!("Note: Mini Git only supports local repository remotes.");
        println!(
            "Setting remote '{}' URL to: {} (for reference only)",
            name, new_url
        );
    } else {
        // Validate that the local path exists and is a mini-git repository
        let path = PathBuf::from(&new_url);
        if !path.exists() {
            return Err(format!("Local path '{}' does not exist", new_url).into());
        }

        let mini_git_dir = path.join(".mini_git");
        if !mini_git_dir.exists() {
            return Err(format!("'{}' is not a Mini Git repository", new_url).into());
        }
    }

    let config_path = repo.git_dir.join("config");
    let config_content = fs::read_to_string(&config_path)?;

    let lines: Vec<&str> = config_content.lines().collect();
    let mut new_lines = Vec::new();
    let mut in_remote_section = false;
    let remote_header = format!("[remote \"{}\"]", name);
    let mut found_remote = false;

    for line in lines {
        let line_trimmed = line.trim();

        if line_trimmed == remote_header {
            in_remote_section = true;
            found_remote = true;
            new_lines.push(line);
            continue;
        }

        if in_remote_section {
            if line_trimmed.starts_with('[') && line_trimmed.ends_with(']') {
                // Entered a new section
                in_remote_section = false;
                new_lines.push(line);
            } else if line_trimmed.starts_with("url = ") {
                // Replace the URL
                // new_lines.push(&format!("\turl = {}", new_url));
                let replaced = format!("\turl = {}", new_url);
                new_lines.push(Box::leak(replaced.into_boxed_str()));
            } else {
                new_lines.push(line);
            }
        } else {
            new_lines.push(line);
        }
    }

    if !found_remote {
        return Err(format!("Remote '{}' not found", name).into());
    }

    fs::write(config_path, new_lines.join("\n"))?;
    println!("Updated remote '{}' URL to: {}", name, new_url);
    Ok(())
}

fn get_remote_url(repo: &Repository, name: String) -> Result<()> {
    let config_path = repo.git_dir.join("config");
    let config_content = fs::read_to_string(&config_path)?;

    let lines: Vec<&str> = config_content.lines().collect();
    let mut in_remote_section = false;
    let remote_header = format!("[remote \"{}\"]", name);

    for line in lines {
        let line_trimmed = line.trim();

        if line_trimmed == remote_header {
            in_remote_section = true;
            continue;
        }

        if in_remote_section {
            if line_trimmed.starts_with('[') && line_trimmed.ends_with(']') {
                break;
            }

            if line_trimmed.starts_with("url = ") {
                let url = line_trimmed.replace("url = ", "");
                println!("{}", url);
                return Ok(());
            }
        }
    }

    Err(format!("Remote '{}' not found", name).into())
}

fn list_remotes(repo: &Repository, verbose: bool) -> Result<()> {
    let config_path = repo.git_dir.join("config");
    if !config_path.exists() {
        println!("No remotes configured");
        return Ok(());
    }

    let config_content = fs::read_to_string(&config_path)?;
    let lines: Vec<&str> = config_content.lines().collect();

    let mut current_remote = None;
    let mut remotes = Vec::new();

    for line in lines {
        let line_trimmed = line.trim();

        if line_trimmed.starts_with("[remote \"") && line_trimmed.ends_with("\"]") {
            let remote_name = line_trimmed.replace("[remote \"", "").replace("\"]", "");
            current_remote = Some(remote_name);
        } else if let Some(ref remote) = current_remote {
            if line_trimmed.starts_with("url = ") {
                let url = line_trimmed.replace("url = ", "");
                remotes.push((remote.clone(), url));
            } else if line_trimmed.starts_with('[') && line_trimmed.ends_with(']') {
                current_remote = None;
            }
        }
    }

    if remotes.is_empty() {
        println!("No remotes configured");
        return Ok(());
    }

    for (name, url) in remotes {
        if verbose {
            let status = if is_local_path(&url) {
                if PathBuf::from(&url).exists() {
                    "(local, available)"
                } else {
                    "(local, not found)"
                }
            } else {
                "(remote, reference only)"
            };

            println!("{}\t{} (fetch) {}", name, url, status);
            println!("{}\t{} (push) {}", name, url, status);
        } else {
            println!("{}", name);
        }
    }

    Ok(())
}

fn is_local_path(url: &str) -> bool {
    // Check if URL is a local file path (not http/https/git/ssh)
    !url.starts_with("http://")
        && !url.starts_with("https://")
        && !url.starts_with("git://")
        && !url.starts_with("ssh://")
        && !url.starts_with("git@")
}
