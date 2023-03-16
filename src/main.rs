use reqwest::{Client, header};
use serde::{Serialize};
use std::env;
use zip::read::ZipArchive;
use std::io::prelude::*;
use std::io::Cursor;

//#[derive(Debug, Deserialize)]
// The RepoTree struct is not used in the current implementation. It may be
// utilized in a future version for a more efficient analysis by providing
// context to the OpenAI API about the repository tree structure.
//
// #[derive(Debug, Deserialize)]
// struct RepoTree {
//     path: String,
// }
#[derive(Serialize)]
struct OpenAiRequest<'a> {
    prompt: &'a str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run <repository_url>");
        return Ok(());
    }

    let repo_url = &args[1];
    let base_url = if repo_url.contains("github.com") {
        "https://api.github.com"
    } else if repo_url.contains("gitlab.com") {
        "https://gitlab.com/api/v4"
    } else {
        panic!("Invalid repository URL. Must be GitHub or GitLab.");
    };

    let client = Client::builder()
        .default_headers(header::HeaderMap::from_iter(vec![
            (header::AUTHORIZATION, format!("Bearer {}", get_token(repo_url)).parse()?),
            (header::USER_AGENT, "rust-app".parse()?),
        ]))
        .build()?;

    let (username, reponame) = parse_repo_url(repo_url)?;
    let repo_zip = fetch_repo_zip(&client, base_url, &username, &reponame).await?;
    let code = download_and_extract_zip(&repo_zip)?;

    // Split the code into chunks to fit within the GPT-3 model's token limit
    let max_chars = 4096; // Adjust this value based on the GPT-3 model's token limit
    let code_chunks = split_code_into_chunks(&code, max_chars);
    let total_parts = code_chunks.len();

    let openai_api_key = "YOUR_OPENAI_API_KEY";
    let mut analysis_results = Vec::new();

    for (i, chunk) in code_chunks.into_iter().enumerate() {
        let prompt = format!("Analyze the following truncated code from the repository at {}. This is part {} of {}:\n\n```\n{}\n```\nDescribe what the software does, and if there's anything suspicious or potentially considered malware.", repo_url, i + 1, total_parts, chunk);

        let openai_result = query_openai_gpt3(&client, openai_api_key, &prompt).await?;
        analysis_results.push(openai_result);
    }

    println!("GPT-3 Analysis:");
    for (i, result) in analysis_results.into_iter().enumerate() {
        println!("Part {} of {}: {}", i + 1, total_parts, result);
    }

    Ok(())
}

fn get_token(repo_url: &str) -> &str {
    if repo_url.contains("github.com") {
        "YOUR_GITHUB_TOKEN"
    } else {
        "YOUR_GITLAB_TOKEN"
    }
}

fn parse_repo_url(repo_url: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut parts = repo_url.split('/');
    let username = parts.next_back().ok_or("Invalid repository URL")?;
    let reponame = parts.next_back().ok_or("Invalid repository URL")?;
    Ok((username.to_string(), reponame.to_string()))
}

async fn fetch_repo_zip(client: &Client, base_url: &str, username: &str, reponame: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let zip_url = if base_url.contains("github.com") {
        format!("{}/repos/{}/{}/zipball", base_url, username, reponame)
    } else {
        format!("{}/projects/{}/{}/repository/archive.zip", base_url, username, reponame)
    };

    let resp = client.get(&zip_url).send().await?;
    let bytes = resp.bytes().await?;
    Ok(bytes.to_vec())
}

async fn query_openai_gpt3(client: &Client, api_key: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://api.openai.com/v1/engines/davinci-codex/completions";
    let req_body = OpenAiRequest { prompt };

    let resp = client.post(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .json(&req_body)
        .send()
        .await?;

    let json_resp: serde_json::Value = resp.json().await?;
    let result = json_resp["choices"][0]["text"].as_str().unwrap_or("").trim().to_string();
    Ok(result)
}

fn download_and_extract_zip(zip_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let reader = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(reader)?;

    let mut code = String::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().ends_with(".zip") {
            continue;
        }

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        code.push_str(&contents);
        code.push_str("\n\n");
    }

    Ok(code)
}

fn split_code_into_chunks(code: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < code.len() {
        let end = (start + max_chars).min(code.len());
        let chunk = &code[start..end];
        chunks.push(chunk.to_string());
        start = end;
    }
    chunks
}
