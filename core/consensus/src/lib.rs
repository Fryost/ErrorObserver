pub struct Candidate {
    pub command: String,
    pub adapted: bool,
    pub has_placeholder: bool,
    pub similarity: f64,
    pub answer_score: i64,
    pub post: String,
}

pub struct Suggestion {
    pub command: String,
    pub weight: f64,
    pub posts: Vec<String>,
    pub adapted: bool,
    pub has_placeholder: bool,
}

pub fn normalize(command: &str) -> String {
    let mut words = Vec::new();
    for word in command.split_whitespace() {
        if word == "apt-get" {
            words.push("apt".to_string());
        } else if word == "-y" || word == "--yes" {
            continue;
        } else {
            words.push(word.trim_end_matches(';').to_string());
        }
    }
    words.join(" ")
}

fn post_weight(candidate: &Candidate) -> f64 {
    let mut score = candidate.answer_score;
    if score < 0 {
        score = 0;
    }
    let mut weight = candidate.similarity * (1.0 + f64::ln(1.0 + score as f64));
    if candidate.has_placeholder {
        weight = weight * 0.5;
    }
    weight
}

pub fn rank(candidates: &Vec<Candidate>) -> Vec<Suggestion> {
    let mut keys: Vec<String> = Vec::new();
    let mut suggestions: Vec<Suggestion> = Vec::new();

    for candidate in candidates {
        let key = normalize(&candidate.command);
        let mut position = None;
        for i in 0..keys.len() {
            if keys[i] == key {
                position = Some(i);
            }
        }

        match position {
            Some(i) => {
                let suggestion = &mut suggestions[i];
                if !suggestion.posts.contains(&candidate.post) {
                    suggestion.posts.push(candidate.post.clone());
                    suggestion.weight += post_weight(candidate);
                }
            }
            None => {
                keys.push(key);
                suggestions.push(Suggestion {
                    command: candidate.command.clone(),
                    weight: post_weight(candidate),
                    posts: vec![candidate.post.clone()],
                    adapted: candidate.adapted,
                    has_placeholder: candidate.has_placeholder,
                });
            }
        }
    }

    suggestions.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(command: &str, post: &str, similarity: f64) -> Candidate {
        Candidate {
            command: command.to_string(),
            adapted: false,
            has_placeholder: false,
            similarity,
            answer_score: 5,
            post: post.to_string(),
        }
    }

    #[test]
    fn groups_apt_and_apt_get() {
        assert_eq!(normalize("sudo apt-get -y update"), normalize("sudo apt update"));
    }

    #[test]
    fn agreement_beats_one_good_match() {
        let candidates = vec![
            candidate("sudo apt-get install mysql-workbench", "a", 1.0),
            candidate("sudo apt update", "b", 0.8),
            candidate("sudo apt-get update", "c", 0.8),
            candidate("sudo apt update", "d", 0.7),
        ];
        let ranked = rank(&candidates);
        assert_eq!(normalize(&ranked[0].command), "sudo apt update");
        assert_eq!(ranked[0].posts.len(), 3);
    }

    #[test]
    fn one_post_counts_once() {
        let candidates = vec![candidate("sudo apt update", "a", 1.0), candidate("sudo apt update", "a", 1.0)];
        let ranked = rank(&candidates);
        assert_eq!(ranked[0].posts.len(), 1);
    }
}