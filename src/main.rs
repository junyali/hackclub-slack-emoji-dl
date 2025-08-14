use clap::Parser;
use std::path::PathBuf;

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

fn main() {
	let args = Args::parse();

	println!("meow :3");
	println!("args: {}, {}, {}", args.output_dir.display(), args.concurrent, args.api_url);
}
