use reqwest::{Client, header};
use serde::Serialize;
use std::env;
use zip::read::ZipArchive;
use std::io::prelude::*;
use std::io::Cursor;
use dotenv::dotenv;
use std::collections::HashSet;

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

#[cfg(test)]
mod tests;

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

    let (code, tech_stack) = download_and_extract_zip(&repo_zip)?;

    let max_chars = 8192; // Increase max_chars to 8192

    let code_chunks = split_code_into_chunks(&code, max_chars);
    let total_parts = code_chunks.len();

    //DEBUG OUTPUT:
    println!("Code Chunks:");
    for (i, chunk) in code_chunks.iter().enumerate() {
        println!("Part {} of {}:\n{}", i + 1, total_parts, chunk);
    }

    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env");

    let mut analysis_results = Vec::new();

    for (i, chunk) in code_chunks.into_iter().enumerate() {
        let prompt = format!("Analyze the following truncated code from the repository at {}. This is part {} of {}. The repository contains files with extensions: {:?}. Please provide an in-depth analysis of the code, its purpose, and if there's anything suspicious or potentially considered malware:\n\n```\n{}\n```\n", repo_url, i + 1, total_parts, tech_stack, chunk);
        println!("Sending request to OpenAI API:");
        println!("Prompt:\n{}", prompt);
        let openai_result = query_openai_gpt3(&client, &openai_api_key, &prompt).await?;
        analysis_results.push(openai_result);
    }

    println!("GPT-3 Analysis:");
    println!("Detected Technology Stack: {:?}", tech_stack);
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
    let status = resp.status(); // Store the status before calling resp.text()

    if status.is_success() {
        let body = resp.text().await?;
        let repo_info: serde_json::Value = serde_json::from_str(&body)?;
        let default_branch = repo_info["default_branch"].as_str().unwrap_or("main").to_string();
        Ok(default_branch)
    } else {
        let body = resp.text().await?;
        Err(format!("Failed to fetch repository information: {} - Response body: {}", status, body).into())
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
        println!("Successfully downloaded archive from: {}", zip_url);
        Ok(bytes.to_vec())
    } else {
        Err(format!("Failed to download archive: {}", resp.status()).into())
    }
}

fn download_and_extract_zip(repo_zip: &[u8]) -> Result<(String, HashSet<String>), Box<dyn std::error::Error>> {
    println!("Extracting ZIP archive...");
    let reader = Cursor::new(repo_zip);
    let mut zip = ZipArchive::new(reader)?;

    let mut code = String::new();
    let mut tech_stack = HashSet::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if let Some(extension) = file_extension(&file.name()) {
            tech_stack.insert(extension.to_string());
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            code.push_str(&contents);
        }
        println!("Processed file: {}", file.name());
    }
    println!("ZIP archive extraction completed.");
    Ok((code, tech_stack))
}

fn file_extension(file_name: &str) -> Option<&str> {
    file_name.split('.').last()
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
    let request = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "system", "content": "You are an AI language model that can analyze codebases and provide insights."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.5,
        "max_tokens": 100
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header(header::AUTHORIZATION, format!("Bearer {}", api_key))
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    let response_text: serde_json::Value = response.json().await?;

    // Debug output for the entire OpenAI API response
    println!("OpenAI API response: {}", response_text);

    // Check if the response contains an "error" key
    if response_text.get("error").is_some() {
        let error_message = response_text["error"]["message"].as_str().unwrap_or("Unknown error");
        return Err(format!("OpenAI API error: {}", error_message).into());
    }

    let result = response_text["choices"][0]["message"]["content"].as_str().unwrap_or_default().trim().to_string();

    Ok(result)
}


