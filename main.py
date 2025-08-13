import logging
import os
import requests
from datetime import datetime
from pathlib import Path

API_URL = "https://badger.hackclub.dev/api/emoji"
OUTPUT_DIR = "./output"

def setup_logging():
	timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
	log_filename = f"download_{timestamp}.log"

	logging.basicConfig(
		level=logging.INFO,
		format="%(asctime)s - %(levelname)s - %(message)s",
		handlers=[
			logging.FileHandler(log_filename, encoding="utf-8"),
			logging.StreamHandler()
		]
	)

	logger = logging.getLogger(__name__)
	return logger

def download_emoji(name, url, logger):
	try:
		sanitised_name = "".join(c for c in name if c.isalnum() or c in ("-", "_"))
		if not sanitised_name:
			sanitised_name = "emoji"

		extension = os.path.splitext(url)[1] or ".png"
		filename = f"{sanitised_name}{extension}"
		filepath = os.path.join(OUTPUT_DIR, filename)

		if os.path.exists(filepath):
			logger.info(f"Skipped {name} (already exists)")
			return True

		response = requests.get(url, timeout=5)
		response.raise_for_status()

		with open(filepath, "wb") as f:
			f.write(response.content)

		logger.info(f"Downloaded {name} -> {filepath}")
		return True
	except Exception as e:
		logger.error(f"Failed to download {name}: {e}")
		return False

def main():
	logger = setup_logging()

	Path(OUTPUT_DIR).mkdir(exist_ok=True)
	logger.info(f"Output directory: {OUTPUT_DIR}")
	logger.info(f"API URL: {API_URL}")

	try:
		logger.info("Fetching data, hang tight!...")
		response = requests.get(API_URL, timeout=30)
		response.raise_for_status()
		emoji_data = response.json()
		logger.info(f"Found {len(emoji_data)} emojis")

		success_count = 0
		for name, url in emoji_data.items():
			if url and download_emoji(name, url, logger):
				success_count += 1

		logger.info(f"Download complete: {success_count} / {len(emoji_data)} successful")
	except Exception as e:
		logger.error(f"An error occurred: {e}")

if __name__ == "__main__":
	main()
