use crate::{Repository, Result, object_store::ObjectStore, utils};
use std::collections::HashMap;
use std::fs;

pub fn diff(repo: &Repository, files: Vec<String>) -> Result<()> {
    let index = utils::load_index(repo)?;
    let object_store = ObjectStore::new(repo);

    if files.is_empty() {
        // Show diff for all tracked files
        for (path, index_entry) in &index.entries {
            show_file_diff(repo, &object_store, path, &index_entry.hash)?;
        }
    } else {
        // Show diff for specific files
        for file in files {
            if let Some(index_entry) = index.entries.get(&file) {
                show_file_diff(repo, &object_store, &file, &index_entry.hash)?;
            } else {
                println!("File '{}' is not tracked", file);
            }
        }
    }

    Ok(())
}

fn show_file_diff(
    repo: &Repository,
    object_store: &ObjectStore,
    path: &str,
    staged_hash: &str,
) -> Result<()> {
    let file_path = repo.work_dir.join(path);

    if !file_path.exists() {
        println!("diff --git a/{} b/{}", path, path);
        println!("deleted file mode 100644");
        println!("index {}..0000000", &staged_hash[..7]);
        println!("--- a/{}", path);
        println!("+++ /dev/null");

        // Show deleted content
        let blob = object_store.load_blob(staged_hash)?;
        let staged_content = String::from_utf8_lossy(&blob.content);
        for (i, line) in staged_content.lines().enumerate() {
            println!("-{}: {}", i + 1, line);
        }
        return Ok(());
    }

    let current_content = fs::read(&file_path)?;
    let current_hash = ObjectStore::hash_content(&current_content);

    if current_hash == staged_hash {
        return Ok(()); // No differences
    }

    let blob = object_store.load_blob(staged_hash)?;
    let staged_content = String::from_utf8_lossy(&blob.content);
    let current_content_str = String::from_utf8_lossy(&current_content);

    println!("diff --git a/{} b/{}", path, path);
    println!("index {}..{} 100644", &staged_hash[..7], &current_hash[..7]);
    println!("--- a/{}", path);
    println!("+++ b/{}", path);

    show_unified_diff(&staged_content, &current_content_str);

    Ok(())
}

fn show_unified_diff(old_content: &str, new_content: &str) {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let diff = compute_diff(&old_lines, &new_lines);

    let mut old_line_num = 1;
    let mut new_line_num = 1;
    let mut i = 0;

    while i < diff.len() {
        // Find the start of a difference block
        if diff[i] != DiffType::Equal {
            let chunk_start = i;

            // Find the end of this difference block
            while i < diff.len() && diff[i] != DiffType::Equal {
                i += 1;
            }

            let chunk_end = i;

            // Calculate line numbers for the chunk
            let old_start = old_line_num;
            let new_start = new_line_num;

            let old_count = diff[chunk_start..chunk_end]
                .iter()
                .filter(|&&d| d == DiffType::Delete || d == DiffType::Equal)
                .count();
            let new_count = diff[chunk_start..chunk_end]
                .iter()
                .filter(|&&d| d == DiffType::Insert || d == DiffType::Equal)
                .count();

            println!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            );

            // Show the actual differences
            for j in chunk_start..chunk_end {
                match diff[j] {
                    DiffType::Delete => {
                        println!("-{}", old_lines[old_line_num - 1]);
                        old_line_num += 1;
                    }
                    DiffType::Insert => {
                        println!("+{}", new_lines[new_line_num - 1]);
                        new_line_num += 1;
                    }
                    DiffType::Equal => {
                        println!(" {}", old_lines[old_line_num - 1]);
                        old_line_num += 1;
                        new_line_num += 1;
                    }
                }
            }
        } else {
            old_line_num += 1;
            new_line_num += 1;
            i += 1;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffType {
    Equal,
    Delete,
    Insert,
}

fn compute_diff(old_lines: &[&str], new_lines: &[&str]) -> Vec<DiffType> {
    // Simple LCS-based diff algorithm
    let mut dp = vec![vec![0; new_lines.len() + 1]; old_lines.len() + 1];

    // Fill the DP table
    for i in 1..=old_lines.len() {
        for j in 1..=new_lines.len() {
            if old_lines[i - 1] == new_lines[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to find the diff
    let mut result = Vec::new();
    let mut i = old_lines.len();
    let mut j = new_lines.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            result.push(DiffType::Equal);
            i -= 1;
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i - 1][j] >= dp[i][j - 1]) {
            result.push(DiffType::Delete);
            i -= 1;
        } else {
            result.push(DiffType::Insert);
            j -= 1;
        }
    }

    result.reverse();
    result
}
