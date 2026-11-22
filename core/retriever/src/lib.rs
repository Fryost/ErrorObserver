use classifier::Classifier;
use std::collections::HashMap;

pub struct Match {
    pub similarity: f64,
    pub error: String,
    pub fix: String,
    pub commands: Vec<String>,
    pub url: String,
    pub answer_score: i64,
}

struct Post {
    vector: HashMap<usize, f64>,
    error: String,
    fix: String,
    commands: Vec<String>,
    url: String,
    answer_score: i64,
}

pub struct Retriever {
    posts: Vec<Post>,
}

fn column(headers: &csv::StringRecord, name: &str) -> Result<usize, String> {
    for i in 0..headers.len() {
        if &headers[i] == name {
            return Ok(i);
        }
    }
    Err(format!("column '{}' is missing, run load_data.py again", name))
}

fn similarity(a: &HashMap<usize, f64>, b: &HashMap<usize, f64>) -> f64 {
    let mut total = 0.0;
    for (index, value) in a {
        if let Some(other) = b.get(index) {
            total += value * other;
        }
    }
    total
}

impl Retriever {
    pub fn load(path: &str, classifier: &Classifier) -> Result<Retriever, String> {
        let mut reader = match csv::Reader::from_path(path) {
            Ok(reader) => reader,
            Err(error) => return Err(format!("{}: {}", path, error)),
        };
        let headers = match reader.headers() {
            Ok(headers) => headers.clone(),
            Err(error) => return Err(format!("{}: {}", path, error)),
        };
        let error_column = column(&headers, "error")?;
        let fix_column = column(&headers, "fix")?;
        let commands_column = column(&headers, "fix_commands")?;
        let url_column = column(&headers, "url")?;
        let score_column = column(&headers, "answer_score")?;

        let mut posts = Vec::new();
        for record in reader.records() {
            let record = match record {
                Ok(record) => record,
                Err(_) => continue,
            };
            let mut commands = Vec::new();
            for line in record[commands_column].lines() {
                if !line.trim().is_empty() {
                    commands.push(line.trim().to_string());
                }
            }
            let post = Post {
                vector: classifier.vectorize(&record[error_column]),
                error: record[error_column].to_string(),
                fix: record[fix_column].to_string(),
                commands,
                url: record[url_column].to_string(),
                answer_score: record[score_column].parse().unwrap_or(0),
            };
            posts.push(post);
        }
        Ok(Retriever { posts })
    }

    pub fn search(&self, classifier: &Classifier, text: &str, count: usize, minimum: f64) -> Vec<Match> {
        let query = classifier.vectorize(text);

        let mut scored: Vec<(f64, usize)> = Vec::new();
        for i in 0..self.posts.len() {
            let value = similarity(&query, &self.posts[i].vector);
            if value >= minimum {
                scored.push((value, i));
            }
        }
        scored.sort_by(|a, b| {
            let by_similarity = b.0.total_cmp(&a.0);
            if by_similarity.is_eq() {
                return self.posts[b.1].answer_score.cmp(&self.posts[a.1].answer_score);
            }
            by_similarity
        });

        let mut matches: Vec<Match> = Vec::new();
        for (value, i) in scored {
            if matches.len() >= count {
                break;
            }
            let post = &self.posts[i];
            let found = Match {
                similarity: value,
                error: post.error.clone(),
                fix: post.fix.clone(),
                commands: post.commands.clone(),
                url: post.url.clone(),
                answer_score: post.answer_score,
            };
            matches.push(found);
        }
        matches
    }
}