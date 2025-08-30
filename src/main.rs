use clap::Parser;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::time::Instant;
use tokio::io::AsyncWriteExt;
use anyhow::{Context, Result};
use tracing::{info, warn, error};
use reqwest::{Client, ClientBuilder};
use std::collections::{HashMap, HashSet};
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

	#[arg(long, default_value = "5000")]
	batch_size: usize,

	#[arg(long, help = "Skip checking for existing files")]
	skip_existence_check: bool,

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
	skip_existence_check: bool,
) -> Result<()> {
	if !url.starts_with("http://") && !url.starts_with("https://") {
		warn!("Skipped {} (invalid URL: {})", name, url);
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

	if !skip_existence_check && filepath.exists() {
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

async fn download_emoji_with_retry(
	client: &Client,
	name: String,
	url: String,
	output_dir: &Path,
	completed: Arc<AtomicUsize>,
	total: usize,
	skip_existence_check: bool,
) -> Result<()> {
	const MAX_RETRIES: usize = 3;

	if !url.starts_with("http://") && !url.starts_with("https://") {
		warn!("Skipped {} (invalid URL: {})", name, url);
		return Ok(());
	}

	let mut last_error = None;

	for attempt in 1..=MAX_RETRIES {
		match download_emoji(client, name.clone(), url.clone(), output_dir.clone(), completed.clone(), total, skip_existence_check).await {
			Ok(()) => return Ok(()),
			Err(e) => {
				if attempt < MAX_RETRIES {
					let backoff = std::time::Duration::from_millis(500 * 2u64.pow((attempt - 1) as u32));
					warn!("Retry {}/{} for {}: {} (waiting {:?})",
						attempt, MAX_RETRIES, name, e, backoff);
					tokio::time::sleep(backoff).await;
					last_error = Some(e);
				} else {
					return Err(e);
				}
			}
		}
	}

	// This should never be reached >:(
	Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error during retry")))
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
	info!("Batch size: {}", args.batch_size);
	info!("API URL: {}", args.api_url);

	let existing_files = if args.skip_existence_check {
		HashSet::new()
	} else {
		info!("Scanning output directory for existing files...");
		let mut files = HashSet::new();
		let mut entries = fs::read_dir(&args.output_dir).await?;
		while let Some(entry) = entries.next_entry().await? {
			if let Ok(file_name) = entry.file_name().into_string() {
				files.insert(file_name);
			}
		}
		info!("Found {} existing files to skip", files.len());
		files
	};

	let client = ClientBuilder::new()
		.pool_max_idle_per_host(args.concurrent)
		.pool_idle_timeout(std::time::Duration::from_secs(30))
		.timeout(std::time::Duration::from_secs(15))
		.tcp_keepalive(std::time::Duration::from_secs(60))
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

	let total_emojis = valid_emojis.len();
	info!("Starting download of {} emojis in batches of {}...", total_emojis, args.batch_size);

	let completed = Arc::new(AtomicUsize::new(0));
	let mut total_processed = 0;
	let mut success_count = 0;

	for(batch_index, batch) in valid_emojis.chunks(args.batch_size).enumerate() {
		info!(
			"Processing batch {}/{} ({} emojis)",
			batch_index + 1,
			(total_emojis + args.batch_size - 1) / args.batch_size,
			batch.len()
		);

		let batch_start = Instant::now();

		let mut results = stream::iter(batch.to_vec())
			.map(|(name, url)| {
				let client = client.clone();
				let output_dir = args.output_dir.clone();
				let completed = completed.clone();
				let existing_files = &existing_files;

				async move {
					if !args.skip_existence_check {
						let sanitised_name = sanitise_filename(&name);
						let sanitised_name = if sanitised_name.is_empty() {
							"emoji".to_string()
						} else {
							sanitised_name
						};

						let extension = extract_extension(&url);
						let filename = format!("{}{}", sanitised_name, extension);

						if existing_files.contains(&filename) {
							completed.fetch_add(1, Ordering::Relaxed);
							return Ok(());
						}
					}

					match download_emoji_with_retry(&client, name.clone(), url, &output_dir, completed, total_emojis, true).await {
						Ok(()) => Ok(()),
						Err(e) => {
							error!("Failed to download {}: {}", name, e);
							Err(e)
						}
					}
				}
			})
			.buffer_unordered(args.concurrent);

		while let Some(result) = results.next().await {
			total_processed += 1;
			if result.is_ok() {
				success_count += 1;
			}
		}

		let batch_elapsed = batch_start.elapsed();
		info!(
			"Batch {}/{} completed in {:.2?} ({} emojis/sec)",
			batch_index + 1,
			(total_emojis + args.batch_size - 1) / args.batch_size,
			batch_elapsed,
			batch.len() as f64 / batch_elapsed.as_secs_f64()
		);

		drop(results);
		tokio::task::yield_now().await;
	}

	let elapsed = start_time.elapsed();
	info!(
		"Download complete: {} / {} successful in {:.2?} ({} emojis/sec)",
		success_count,
		total_processed,
		elapsed,
		total_processed as f64 / elapsed.as_secs_f64()
	);

	println!("\nPress Enter to exit...");
	let mut input = String::new();
	std::io::stdin().read_line(&mut input).expect("Failed to read input");

	Ok(())
}
