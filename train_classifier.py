import json
from pathlib import Path

import pandas as pd
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import classification_report

ROOT = Path(__file__).parent
OUTPUT = ROOT / "output"
MODELS = ROOT / "models"
NGRAM_MAX = 2


def export_model(vectorizer, model):
	vocabulary = {}
	for term in vectorizer.vocabulary_:
		vocabulary[term] = int(vectorizer.vocabulary_[term])

	exported = {
		"classes": list(model.classes_),
		"vocabulary": vocabulary,
		"idf": vectorizer.idf_.tolist(),
		"coef": model.coef_.tolist(),
		"intercept": model.intercept_.tolist(),
		"ngram_max": NGRAM_MAX,
	}
	MODELS.mkdir(exist_ok=True)
	(MODELS / "classifier.json").write_text(json.dumps(exported))


def export_parity_check(vectorizer, model, test):
	samples = test["error"].head(20).tolist()
	probabilities = model.predict_proba(vectorizer.transform(samples))

	cases = []
	for i in range(len(samples)):
		cases.append({"text": samples[i], "probabilities": probabilities[i].tolist()})
	(MODELS / "parity.json").write_text(json.dumps(cases))


def main():
	train = pd.read_csv(OUTPUT / "train.csv")
	test = pd.read_csv(OUTPUT / "test.csv")

	vectorizer = TfidfVectorizer(lowercase=True, token_pattern=r"(?u)\b\w\w+\b", ngram_range=(1, NGRAM_MAX), min_df=2, max_features=20000)
	x_train = vectorizer.fit_transform(train["error"])
	x_test = vectorizer.transform(test["error"])

	model = LogisticRegression(max_iter=2000, class_weight="balanced")
	model.fit(x_train, train["category"])

	predicted = model.predict(x_test)
	print(classification_report(test["category"], predicted, zero_division=0))

	results = test[["error", "category"]].copy()
	results["predicted"] = predicted
	results["correct"] = results["category"] == results["predicted"]
	results.to_csv(OUTPUT / "predictions.csv", index=False)

	export_model(vectorizer, model)
	export_parity_check(vectorizer, model, test)
	print("Saved", MODELS / "classifier.json")


if __name__ == "__main__":
	main()