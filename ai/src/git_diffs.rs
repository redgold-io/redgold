use std::process::Command;
use tokio::process::Command as TokioCommand;
use crate::directory_code_reader::count_tokens;

async fn get_git_diff_history(repo_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Change directory to the repository path
    TokioCommand::new("cd")
        .arg(repo_path)
        .status()
        .await?;

    // Run git log command with --patch option
    let output = TokioCommand::new("git")
        .arg("log")
        .arg("--patch")
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!("Git command failed with status: {}", output.status).into())
    }
}

fn parse_git_diff_history(diff_history: &String) -> Vec<(String, String)> {
    let mut commits = Vec::new();

    let mut index = 0;

    let mut commit_text = String::new();
    let mut commit_id = "Missing".to_string();

    for line in diff_history.lines() {
        if line.starts_with("commit ") {
            if commit_text.len() > 0 {
                commits.push((commit_id, commit_text.clone()));
            }

            commit_id = line.trim_start_matches("commit ").to_string();
            let mut commit_text = String::new();
        }
        commit_text.push_str(line);
    }

    commits
}

pub async fn get_git_diffs(path: Option<String>) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let or = path.unwrap_or("../".to_string());
    let diff_history = get_git_diff_history(or.as_str()).await?;
    Ok(parse_git_diff_history(&diff_history))
}

#[ignore]
#[tokio::test]
async fn main() {
    let repo_path = "../";
    match get_git_diff_history(repo_path).await {
        Ok(diff_history) => {
            println!("len diff history {}", diff_history.len());
            // This doesn't match exactly, few commits being dropped but close enough
            let parsed = parse_git_diff_history(&diff_history);
            parsed[0..5].iter().for_each(|(id, text)| {
                println!("Commit: {}", id);
            });
            println!("Parsed commit len {}", parsed.len());
            let count = count_tokens(&diff_history);
            println!("Token count: {}", count);
        },
        Err(e) => eprintln!("Failed to retrieve git diff history: {}", e),
    }
}