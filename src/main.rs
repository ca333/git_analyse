use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::env;
use zip::read::ZipArchive;
use std::io::prelude::*;
use std::io::Cursor;
use dotenv::dotenv;

//#[derive(Debug, Deserialize)]
// The RepoTree struct is not used in the current implementation. It may be
// utilized in a future version for a more efficient analysis by providing
// context to the OpenAI API about the repository tree structure.
//
// #[derive(Debug, Deserialize)]
// struct RepoTree {
//     path: String,
// }

// #[derive(Deserialize)]
// struct RepoInfo {
//     default_branch: String,
// }

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    prompt: &'a str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run <repository_url> [branch]");
        return Ok(());
    }

    let repo_url = &args[1];
    let (username, reponame, _) = parse_repo_url(repo_url)?;

    let client = Client::builder()
        .default_headers(header::HeaderMap::from_iter(vec![
            (header::USER_AGENT, "rust-app".parse()?),
        ]))
        .build()?;

        let branch = if args.len() == 3 {
            args[2].clone()
        } else {
            get_default_branch(&client, repo_url).await?
        };

    let repo_zip = fetch_repo_zip(&client, repo_url, &username, &reponame, &branch).await.unwrap_or_else(|err| {
        eprintln!("Error fetching repository archive: {}", err);
        std::process::exit(1);
    });

    let code = download_and_extract_zip(&repo_zip)?;

    let max_chars = 4096;
    let code_chunks = split_code_into_chunks(&code, max_chars);
    let total_parts = code_chunks.len();

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env");

    let mut analysis_results = Vec::new();

    for (i, chunk) in code_chunks.into_iter().enumerate() {
        let prompt = format!("Analyze the following truncated code from the repository at {}. This is part {} of {}:\n\n```\n{}\n```\nDescribe what the software does, and if there's anything suspicious or potentially considered malware.", repo_url, i + 1, total_parts, chunk);

        let openai_result = query_openai_gpt3(&client, &openai_api_key, &prompt).await?;
        analysis_results.push(openai_result);
    }

    println!("GPT-3 Analysis:");
    for (i, result) in analysis_results.into_iter().enumerate() {
        println!("Part {} of {}: {}", i + 1, total_parts, result);
    }

    Ok(())
}

async fn get_default_branch(client: &Client, repo_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let api_url = if repo_url.contains("github.com") {
        let (username, reponame, _) = parse_repo_url(repo_url)?;
        format!("https://api.github.com/repos/{}/{}", username, reponame)
    } else {
        panic!("Only GitHub is currently supported for fetching the default branch.");
    };

    let resp = client.get(&api_url).send().await?;
    if resp.status().is_success() {
        let repo_info: serde_json::Value = resp.json().await?;
        let default_branch = repo_info["default_branch"].as_str().unwrap_or("master").to_string();
        Ok(default_branch)
    } else {
        Err(format!("Failed to fetch repository information: {}", resp.status()).into())
    }
}

fn parse_repo_url(repo_url: &str) -> Result<(String, String, Option<String>), Box<dyn std::error::Error>> {
    let mut parts = repo_url.split('/');
    let reponame = parts.next_back().ok_or("Invalid repository URL")?;
    let username = parts.next_back().ok_or("Invalid repository URL")?;
    let branch = parts.next_back().map(|s| s.to_string());
    Ok((username.to_string(), reponame.to_string(), branch))
}

async fn fetch_repo_zip(client: &Client, repo_url: &str, username: &str, reponame: &str, branch: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let zip_url = if repo_url.contains("github.com") {
        format!("https://github.com/{}/{}/archive/{}.zip", username, reponame, branch)
    } else if repo_url.contains("gitlab.com") {
        format!("https://gitlab.com/{}/{}/-/archive/{}/{}-{}.zip", username, reponame, branch, reponame, branch)
    } else {
        panic!("Invalid repository URL. Must be GitHub or GitLab.");
    };

    let resp = client.get(&zip_url).send().await?;
    if resp.status().is_success() {
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    } else {
        Err(format!("Failed to download archive: {}", resp.status()).into())
    }
}

fn download_and_extract_zip(repo_zip: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let reader = Cursor::new(repo_zip);
    let mut zip = ZipArchive::new(reader)?;

    let mut code = String::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with(".rs") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            code.push_str(&contents);
        }
    }
    Ok(code)
}

fn split_code_into_chunks(code: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < code.len() {
        let end = (start + max_chars).min(code.len());
        let slice = &code[start..end];
        chunks.push(slice.to_string());
        start = end;
    }

    chunks // Returns the 'chunks' vector containing the code segments.
}

async fn query_openai_gpt3(client: &Client, api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request = OpenAiRequest { prompt };
    let response = client
        .post("https://api.openai.com/v1/engines/davinci-codex/completions")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    let response_text: serde_json::Value = response.json().await?;
    let result = response_text["choices"][0]["text"].as_str().unwrap_or_default().trim().to_string();

    Ok(result)
}