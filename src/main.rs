use clap::Parser;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::time::Instant;
use tokio::io::AsyncWriteExt;
use anyhow::{Context, Result};
use tracing::{info, warn, error};
use reqwest::{Client, ClientBuilder};
use std::collections::HashMap;
use serde_json::Value;
use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "hackclub-slack-emoji-dl")]
#[command(about = "Download Hack Club Slack emojis")]
struct Args {
	#[arg(short, long, default_value = "./output")]
	output_dir: PathBuf,

	#[arg(short, long, default_value = "500")]
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

async fn download_emoji(
	client: &Client,
	name: String,
	url: String,
	output_dir: &Path,
	completed: Arc<AtomicUsize>,
	total: usize,
) -> Result<()> {
	if !url.starts_with("http://") && !url.starts_with("https://") {
		warn!("Skipped {} (invalid URL)", name);
		return Ok (());
	}

	let sanitised_name = sanitise_filename(&name);
	let sanitised_name = if sanitised_name.is_empty() {
		"emoji".to_string()
	} else {
		sanitised_name
	};

	let extension = extract_extension(&url);
	let filename = format!("{}{}", sanitised_name, extension);
	let filepath = output_dir.join(filename);

	if filepath.exists() {
		info!("Skipped {} (already exists)", name);
		return Ok(());
	}

	let response = client
		.get(&url)
		.timeout(std::time::Duration::from_secs(10))
		.send()
		.await
		.context(format!("Failed to fetch {}", url))?;

	if !response.status().is_success() {
		return Err(anyhow::anyhow!(
			"HTTP error {} for {}",
			response.status(),
			name
		))
	}

	let bytes = response
		.bytes()
		.await
		.context("Failed to read response body")?;

	let mut file = fs::File::create(&filepath)
		.await
		.context(format!("Failed to create file {}", filepath.display()))?;

	file.write_all(&bytes)
		.await
		.context(format!("Failed to write data to {}", filepath.display()))?;

	file.flush()
		.await
		.context(format!("Failed to flush data to {}", filepath.display()))?;
	let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
	info!("Downloaded {} -> {} [{}/{}]", name, filepath.display(), current, total);
	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let args = Args::parse();
	let start_time = Instant::now();

	println!("meow :3");

	fs::create_dir_all(&args.output_dir)
		.await
		.context("Failed to create output directory")?;

	info!("Output directory: {}", args.output_dir.display());
	info!("Concurrent downloads: {}", args.concurrent);
	info!("API URL: {}", args.api_url);

	let client = ClientBuilder::new()
		.pool_max_idle_per_host(args.concurrent)
		.pool_idle_timeout(std::time::Duration::from_secs(30))
		.timeout(std::time::Duration::from_secs(15))
		.build()
		.context("Failed to create HTTP client")?;

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

	let total_emojis = valid_emojis.len();
	let completed = Arc::new(AtomicUsize::new(0));

	let mut results = stream::iter(valid_emojis)
		.map(|(name, url)| {
			let client = client.clone();
			let output_dir = args.output_dir.clone();
			let completed = completed.clone();
			async move {
				match download_emoji(&client, name.clone(), url, &output_dir, completed, total_emojis).await {
					Ok(()) => {
						Ok(())
					}
					Err(e) => {
						error!("Failed to download {}: {}", name, e);
						Err(e)
					}
				}
			}
		})
		.buffer_unordered(args.concurrent);

	let mut total_processed = 0;
	let mut success_count = 0;
	while let Some(result) = results.next().await {
		total_processed += 1;
		if result.is_ok() {
			success_count += 1;
		}
	}

	let elapsed = start_time.elapsed();
	info!("Download complete: {} / {} successful in {:.2?}", success_count, total_processed, elapsed);

	println!("\nPress Enter to exit...");
	let mut input = String::new();
	std::io::stdin().read_line(&mut input).expect("Failed to read input");

	Ok(())
}
