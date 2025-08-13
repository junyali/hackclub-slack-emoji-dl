import logging
from datetime import datetime

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

def main():
	logger = setup_logging()

if __name__ == "__main__":
	main()
