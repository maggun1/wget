use clap::{Arg, ArgAction, Command};
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use select::document::Document;
use select::predicate::{Attr, Name, Predicate};
use std::collections::HashSet;
use std::fs::{create_dir_all, write};
use std::path::{Path, PathBuf};
use url::Url;

fn download(
    url: &Url,
    client: &Client,
    visited: &mut HashSet<String>,
    recursive: bool,
    base_host: &str,
    output_dir: &Path)
    -> Result<(), Box<dyn std::error::Error>> {
    if visited.contains(url.as_str()) {
        println!("Already visited: {}", url);
        return Ok(());
    }

    visited.insert(url.as_str().to_string());

    println!("Downloading: {}", url);
    let response = client
        .get(url.as_str())
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36")
        .send()?
        .text()?;

    let file_path = create_file_path(url, output_dir)?;
    save_content(&file_path, &response)?;

    if recursive {
        println!("Recursive mode is enabled, searching for links and resources...");
        let document = Document::from(response.as_str());

        let resource_tags = vec![
            ("a", "href"),
            ("img", "src"),
            ("script", "src"),
            ("link", "href"),
        ];

        for (tag, attr) in resource_tags {
            for node in document.find(Name(tag).and(Attr(attr, ()))) {
                if let Some(link) = node.attr(attr) {
                    if let Ok(next_url) = url.join(link) {
                        if next_url.host_str() == Some(base_host) && next_url.scheme().starts_with("http") {
                            println!("Found resource link: {}", next_url);
                            download(&next_url, client, visited, recursive, base_host, output_dir)?;
                        } else {
                            println!("Skipping external resource: {}", next_url);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn create_file_path(url: &Url, output_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let file_name = url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("index.html");

    let file_name = if file_name.is_empty() {
        "index.html"
    } else if !file_name.contains('.') {
        &format!("{}.html", file_name)
    } else {
        file_name.split('?').next().unwrap()
    };

    let mut file_path = PathBuf::from(output_dir);
    if let Some(segments) = url.path_segments() {
        for segment in segments {
            file_path.push(segment);
        }
    }

    file_path.push(file_name);

    if let Some(parent_dir) = file_path.parent() {
        if !parent_dir.exists() {
            println!("Creating directory: {:?}", parent_dir);
            create_dir_all(parent_dir)?;
        }
    }

    Ok(file_path)
}

fn save_content(file_path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Saving file to: {:?}", file_path);
    write(&file_path, content)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("wget")
        .arg(Arg::new("url")
            .required(true)
            .num_args(1)
            .index(1))

        .arg(Arg::new("output-dir")
            .num_args(1)
            .index(2)
            .default_value("."))

        .arg(Arg::new("recursive")
            .short('r')
            .action(ArgAction::SetTrue))
        .get_matches();

    let url = matches.get_one::<String>("url").unwrap();
    let recursive = matches.get_flag("recursive");
    let output_dir = matches.get_one::<String>("output-dir").unwrap();

    let output_dir = Path::new(output_dir);
    if !output_dir.exists() {
        println!("Creating output directory: {:?}", output_dir);
        create_dir_all(output_dir)?;
    }

    println!("Starting download for: {}", url);
    println!("Recursive mode: {}", recursive);
    println!("Output directory: {:?}", output_dir);

    let client = Client::new();
    let mut visited = HashSet::new();

    let url = Url::parse(url)?;
    let base_host = url.host_str().unwrap_or("");

    download(&url, &client, &mut visited, recursive, base_host, output_dir)?;

    println!("Download completed.");
    Ok(())
}
