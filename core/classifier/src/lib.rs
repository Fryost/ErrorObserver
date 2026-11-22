use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize)]
struct ModelFile {
    classes: Vec<String>,
    vocabulary: HashMap<String, usize>,
    idf: Vec<f64>,
    coef: Vec<Vec<f64>>,
    intercept: Vec<f64>,
    ngram_max: usize,
}

pub struct Prediction {
    pub category: String,
    pub confidence: f64,
}

pub struct Classifier {
    model: ModelFile,
    word_pattern: Regex,
}

impl Classifier {
    pub fn load(path: &str) -> Result<Classifier, String> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => return Err(format!("{}: {}", path, error)),
        };
        let model: ModelFile = match serde_json::from_str(&text) {
            Ok(model) => model,
            Err(error) => return Err(format!("{}: {}", path, error)),
        };
        let word_pattern = Regex::new(r"\b\w\w+\b").unwrap();
        Ok(Classifier { model, word_pattern })
    }

    pub fn classes(&self) -> &Vec<String> {
        &self.model.classes
    }

    fn split_into_words(&self, text: &str) -> Vec<String> {
        let lower = text.to_lowercase();
        let mut words = Vec::new();
        for found in self.word_pattern.find_iter(&lower) {
            words.push(found.as_str().to_string());
        }
        words
    }

    pub fn unknown_words(&self, text: &str) -> Vec<String> {
        let mut unknown = Vec::new();
        for word in self.split_into_words(text) {
            if !self.model.vocabulary.contains_key(&word) && !unknown.contains(&word) {
                unknown.push(word);
            }
        }
        unknown
    }

    pub fn vectorize(&self, text: &str) -> HashMap<usize, f64> {
        let words = self.split_into_words(text);
        let mut features: HashMap<usize, f64> = HashMap::new();

        for size in 1..=self.model.ngram_max {
            if words.len() < size {
                break;
            }
            for start in 0..=(words.len() - size) {
                let phrase = words[start..start + size].join(" ");
                if let Some(index) = self.model.vocabulary.get(&phrase) {
                    match features.get_mut(index) {
                        Some(count) => *count += 1.0,
                        None => {
                            features.insert(*index, 1.0);
                        }
                    }
                }
            }
        }

        let mut sum_of_squares = 0.0;
        for (index, value) in features.iter_mut() {
            *value = *value * self.model.idf[*index];
            sum_of_squares += *value * *value;
        }

        let length = f64::sqrt(sum_of_squares);
        if length > 0.0 {
            for value in features.values_mut() {
                *value = *value / length;
            }
        }
        features
    }

    pub fn probabilities(&self, text: &str) -> Vec<f64> {
        let features = self.vectorize(text);

        let mut scores = Vec::new();
        for row in 0..self.model.coef.len() {
            let mut score = self.model.intercept[row];
            for (index, value) in &features {
                score += self.model.coef[row][*index] * value;
            }
            scores.push(score);
        }

        if scores.len() == 1 {
            let p = 1.0 / (1.0 + f64::exp(-scores[0]));
            return vec![1.0 - p, p];
        }

        let mut highest = scores[0];
        for score in &scores {
            if *score > highest {
                highest = *score;
            }
        }

        let mut total = 0.0;
        let mut probabilities = Vec::new();
        for score in &scores {
            let value = f64::exp(score - highest);
            total += value;
            probabilities.push(value);
        }
        for value in probabilities.iter_mut() {
            *value = *value / total;
        }
        probabilities
    }

    pub fn predict(&self, text: &str) -> Prediction {
        let probabilities = self.probabilities(text);
        let mut best = 0;
        for i in 1..probabilities.len() {
            if probabilities[i] > probabilities[best] {
                best = i;
            }
        }
        Prediction {
            category: self.model.classes[best].clone(),
            confidence: probabilities[best],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Case {
        text: String,
        probabilities: Vec<f64>,
    }

    #[test]
    fn matches_python() {
        let models = format!("{}/../../models", env!("CARGO_MANIFEST_DIR"));
        let model = match Classifier::load(&format!("{}/classifier.json", models)) {
            Ok(model) => model,
            Err(_) => return,
        };
        let text = match fs::read_to_string(format!("{}/parity.json", models)) {
            Ok(text) => text,
            Err(_) => return,
        };
        let cases: Vec<Case> = serde_json::from_str(&text).unwrap();
        for case in cases {
            let ours = model.probabilities(&case.text);
            for i in 0..ours.len() {
                let difference = (ours[i] - case.probabilities[i]).abs();
                assert!(difference < 1e-6, "{}: {} vs {}", case.text, ours[i], case.probabilities[i]);
            }
        }
    }
}