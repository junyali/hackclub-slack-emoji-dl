use clap::Parser;
use std::path::PathBuf;
use tokio::fs;
use anyhow::{Context, Result};
use tracing::{info, warn, error};

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

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	println!("meow :3");

	fs::create_dir_all(&args.output_dir)
		.await
		.context("Failed to create output directory")?;

	info!("Output directory: {}", args.output_dir.display());
	info!("Concurrent downloads: {}", args.concurrent);
	info!("API URL: {}", args.api_url);

	Ok(())
}
