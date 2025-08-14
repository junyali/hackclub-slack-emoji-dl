use clap::Parser;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::time::Instant;
use anyhow::{Context, Result};
use tracing::{info, warn, error};
use reqwest::Client;
use std::collections::HashMap;
use serde_json::Value;

#[derive(Parser)]
#[command(name = "hackclub-slack-emoji-dl")]
#[command(about = "Download Hack Club Slack emojis")]
struct Args {
	#[arg(short, long, default_value = "./output")]
	output_dir: PathBuf,

	#[arg(short, long, default_value = "100")]
	concurrent: usize,

	#[arg(long, default_value = "https://badger.hackclub.dev/api/emoji")]
	api_url: String,
}

fn sanitise_filename(name: &str) -> String {
	name.chars()
		.map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
		.collect::<String>()
		.trim_matches('_')
		.to_string()
}

fn extract_extension(url: &str) -> String {
	Path::new(url)
		.extension()
		.and_then(|ext| ext.to_str())
		.map(|ext| if ext.starts_with('.') { ext.to_string() } else { format!(".{}", ext) })
		.unwrap_or_else(|| ".png".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();
	let start_time = Instant::now();

	println!("meow :3");

	fs::create_dir_all(&args.output_dir)
		.await
		.context("Failed to create output directory")?;

	info!("Output directory: {}", args.output_dir.display());
	info!("Concurrent downloads: {}", args.concurrent);
	info!("API URL: {}", args.api_url);

	let client = Client::new();

	info!("Fetching data, hang tight!");
	let response = client
		.get(&args.api_url)
		.timeout(std::time::Duration::from_secs(10))
		.send()
		.await
		.context("Failed to fetch data")?;

	let emoji_data: HashMap<String, Value> = response
		.json()
		.await
		.context("Failed to parse JSON response")?;

	info!("Found {} emojis", emoji_data.len());

	let valid_emojis: Vec<(String, String)> = emoji_data
		.into_iter()
		.filter_map(|(name, url)| {
			url.as_str()
				.filter(|s| !s.is_empty())
				.map(|s| (name, s.to_string()))
		})
		.collect();

	info!("Starting concurrent download of {} emojis...", valid_emojis.len());

	let elapsed = start_time.elapsed();

	Ok(())
}
