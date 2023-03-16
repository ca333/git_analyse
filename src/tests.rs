use crate::{file_extension, parse_repo_url, split_code_into_chunks, get_default_branch};
use reqwest::Client;

#[tokio::test]
async fn test_get_default_branch() {
    let client = Client::builder()
        .user_agent("git_analyse")
        .build()
        .expect("Failed to build reqwest::Client");
    let repo_url = "https://github.com/ca333/git_analyse";
    let default_branch = get_default_branch(&client, repo_url).await.unwrap();
    assert_eq!(default_branch, "main");
}

#[test]
fn test_parse_repo_url() {
    let repo_url = "https://github.com/ca333/git_analyse";
    let result = parse_repo_url(repo_url).unwrap();
    assert_eq!(("ca333".to_string(), "git_analyse".to_string(), Some("github.com".to_string())), (result.0, result.1, result.2));
}

#[test]
fn test_file_extension() {
    let file_name = "main.rs";
    let ext = file_extension(file_name);
    assert_eq!(ext, Some("rs"));
}

#[test]
fn test_split_code_into_chunks() {
    let code = "A".repeat(10000);
    let chunks = split_code_into_chunks(&code, 3000);
    assert_eq!(chunks.len(), 4);
}
