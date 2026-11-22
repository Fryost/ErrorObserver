import re
import sys
from pathlib import Path

import pandas as pd
from bs4 import BeautifulSoup
from sklearn.model_selection import train_test_split

DATASET = "hf://datasets/HuggingFaceH4/stack-exchange-preferences/data/askubuntu.com/"
FILES = [
	"train-00000-of-00005.parquet",
	"train-00001-of-00005.parquet",
	"train-00002-of-00005.parquet",
	"train-00003-of-00005.parquet",
	"train-00004-of-00005.parquet",
]

ERROR_PATTERN = re.compile(
	r"error|failed|failure|cannot|can't|unable to|not found|no such file|denied|segmentation fault|"
	r"unmet dependencies|broken packages|E: |fatal|exception|traceback|refused|timed out|no space left",
	re.IGNORECASE,
)

CATEGORIES = {
	"permissions": r"permission denied|operation not permitted|are you root|EACCES|not in the sudoers",
	"disk_space": r"no space left|disk full|not enough (free )?space|quota exceeded",
	"packages": r"unmet dependencies|broken packages|unable to locate package|dpkg|apt-get|apt |E: |held broken|has no installation candidate",
	"missing_file": r"no such file or directory|command not found|cannot open shared object|not found",
	"network": r"connection refused|could not resolve|network is unreachable|timed out|temporary failure in name resolution|ssl",
	"boot": r"grub|initramfs|kernel panic|boot|uefi|fstab",
	"display": r"xorg|nvidia|display|screen|wayland|gdm|lightdm",
	"crash": r"segmentation fault|core dumped|crash|traceback|exception",
}


def html_to_text(html):
	if html is None:
		html = ""
	soup = BeautifulSoup(html, "html.parser")
	return soup.get_text(" ", strip=True)


def code_text(html):
	if html is None:
		html = ""
	soup = BeautifulSoup(html, "html.parser")
	pieces = []
	for block in soup.find_all(["pre", "code"]):
		inside_pre = block.name == "code" and block.find_parent("pre") is not None
		if not inside_pre:
			pieces.append(block.get_text())
	return "\n".join(pieces)


COMMAND_PATTERN = re.compile(r"^(sudo\s+)?[a-z0-9][\w.+-]*(\s|$)")


def answer_commands(html):
	if html is None:
		html = ""
	soup = BeautifulSoup(html, "html.parser")
	commands = []
	for block in soup.find_all(["pre", "code"]):
		inside_pre = block.name == "code" and block.find_parent("pre") is not None
		if inside_pre:
			continue
		for line in block.get_text().splitlines():
			line = line.strip()
			if line.startswith("$ "):
				line = line[2:].strip()
			if len(line) < 3 or len(line) > 150:
				continue
			if len(line.split()) < 2:
				continue
			if not COMMAND_PATTERN.match(line):
				continue
			if line not in commands:
				commands.append(line)
		if len(commands) >= 3:
			break
	return "\n".join(commands[:3])


def best_answer(answers):
	best = None
	for answer in answers:
		if answer["selected"]:
			return answer
		if best is None or answer["pm_score"] > best["pm_score"]:
			best = answer
	return best


def error_lines(text):
	hits = []
	for line in text.splitlines():
		if ERROR_PATTERN.search(line):
			hits.append(line.strip())
	joined = " ".join(hits)
	return joined[:500]


def rough_category(text):
	for name in CATEGORIES:
		if re.search(CATEGORIES[name], text, re.IGNORECASE):
			return name
	return "other"


def read_files(files):
	tables = []
	for name in files:
		table = pd.read_parquet(DATASET + name, columns=["qid", "question", "answers", "metadata"])
		tables.append(table)
	return pd.concat(tables)


def load(files):
	raw = read_files(files)
	print(len(raw), "questions loaded")

	rows = []
	for q in raw.itertuples():
		code = code_text(q.question)
		if not ERROR_PATTERN.search(code):
			continue

		answer = best_answer(q.answers)
		if answer is None or answer["pm_score"] < 1:
			continue

		error = error_lines(code)
		if error == "":
			continue

		url = ""
		if len(q.metadata) > 0:
			url = q.metadata[0]

		row = {
			"qid": q.qid,
			"error": error,
			"question": html_to_text(q.question)[:1000],
			"fix": html_to_text(answer["text"])[:1500],
			"fix_commands": answer_commands(answer["text"]),
			"answer_score": answer["pm_score"],
			"accepted": answer["selected"],
			"category": rough_category(error),
			"url": url,
		}
		rows.append(row)

	return pd.DataFrame(rows)


def main():
	file_count = 1
	if len(sys.argv) > 1:
		file_count = int(sys.argv[1])

	posts = load(FILES[:file_count])
	print(len(posts), "questions with error messages and a useful answer")

	errors = posts.drop_duplicates(subset="error")
	print(len(errors), "different error messages")
	print(errors["category"].value_counts())

	train, test = train_test_split(errors, test_size=0.2, random_state=42, stratify=errors["category"])

	output = Path(__file__).parent / "output"
	output.mkdir(exist_ok=True)
	posts.to_csv(output / "errors.csv", index=False)
	train.to_csv(output / "train.csv", index=False)
	test.to_csv(output / "test.csv", index=False)

	counts = errors["category"].value_counts().reset_index()
	counts.columns = ["category", "count"]
	counts.to_csv(output / "categories.csv", index=False)

	print(len(train), "train /", len(test), "test")


if __name__ == "__main__":
	main()