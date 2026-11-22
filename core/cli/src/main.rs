use classifier::Classifier;
use consensus::Candidate;
use fixes::Rules;
use retriever::Retriever;
use safety::{Level, SafetyChecker, Verdict};
use translator::{NotYetAvailable, Translator};

const CANDIDATES: usize = 30;
const MINIMUM_SIMILARITY: f64 = 0.35;
const SHOWN_SUGGESTIONS: usize = 5;
const MINIMUM_POSTS: usize = 2;
const MINIMUM_SHARE: f64 = 0.1;

fn describe(verdict: &Verdict) -> String {
    match verdict.level {
        Level::Safe => "safe".to_string(),
        Level::Warn => format!("careful: {}", verdict.reason),
        Level::Block => format!("blocked: {}", verdict.reason),
    }
}

fn print_general_fixes(rules: &Rules, safety: &SafetyChecker, category: &str) {
    let fixes = rules.for_category(category);
    if fixes.is_empty() {
        println!("  No general fixes for {} yet.", category);
        return;
    }
    for fix in fixes {
        let verdict = safety.check(&fix.command);
        if verdict.level == Level::Block {
            continue;
        }
        println!("  $ {}   [{}]", fix.command, describe(&verdict));
        println!("      {}", fix.description);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: errorobserver \"<error message>\"");
        std::process::exit(2);
    }
    let input = args[1..].join(" ");

    let root = format!("{}/../..", env!("CARGO_MANIFEST_DIR"));
    let classifier = match Classifier::load(&format!("{}/models/classifier.json", root)) {
        Ok(classifier) => classifier,
        Err(error) => {
            println!("Could not load the model ({}). Run train_classifier.py first.", error);
            std::process::exit(1);
        }
    };
    let retriever = match Retriever::load(&format!("{}/output/errors.csv", root), &classifier) {
        Ok(retriever) => Some(retriever),
        Err(error) => {
            println!("Similar-post search is off ({}).", error);
            None
        }
    };
    let rules = Rules::load_builtin();
    let safety = SafetyChecker::new();
    let translator = NotYetAvailable;

    let prediction = classifier.predict(&input);
    let percent = (prediction.confidence * 100.0).round();
    println!("Category: {} ({}% sure)", prediction.category, percent);
    if prediction.confidence < 0.5 {
        println!("Not very sure, this may not be a known kind of error.");
    }

    let unknown = classifier.unknown_words(&input);
    if !unknown.is_empty() {
        println!("Never seen in the data: {} (matching uses the rest of your message)", unknown.join(", "));
    }

    let mut matches = Vec::new();
    if let Some(retriever) = &retriever {
        matches = retriever.search(&classifier, &input, CANDIDATES, MINIMUM_SIMILARITY);
    }

    let mut candidates = Vec::new();
    let mut hidden = 0;
    let mut safest_post: Option<String> = None;
    for found in &matches {
        let differences = adapter::compare(&found.error, &input);
        let mut post_is_safe = true;
        for command in &found.commands {
            let adapted = adapter::adapt(command, &differences);
            let original_blocked = safety.check(command).level == Level::Block;
            let adapted_blocked = safety.check(&adapted.command).level == Level::Block;
            if original_blocked || adapted_blocked {
                hidden += 1;
                post_is_safe = false;
                continue;
            }
            candidates.push(Candidate {
                command: adapted.command,
                adapted: adapted.changed,
                has_placeholder: adapted.has_placeholder,
                similarity: found.similarity,
                answer_score: found.answer_score,
                post: found.url.clone(),
            });
        }
        if post_is_safe && safest_post.is_none() {
            safest_post = Some(found.url.clone());
        }
    }

    let share_needed = (matches.len() as f64 * MINIMUM_SHARE).ceil() as usize;
    let mut needed = MINIMUM_POSTS;
    if share_needed > needed {
        needed = share_needed;
    }

    let ranked = consensus::rank(&candidates);
    let mut suggestions = Vec::new();
    for suggestion in ranked {
        if suggestion.posts.len() >= needed {
            suggestions.push(suggestion);
        }
    }
    let strong = !suggestions.is_empty();

    println!();
    if matches.is_empty() {
        println!("No similar problems found on Ask Ubuntu.");
    } else if !strong {
        println!("Found {} similar Ask Ubuntu posts, but they don't agree on a fix.", matches.len());
    } else {
        println!("Suggested fixes, based on {} similar Ask Ubuntu posts:", matches.len());
        let mut shown = 0;
        for suggestion in &suggestions {
            if shown >= SHOWN_SUGGESTIONS {
                break;
            }
            let verdict = safety.check(&suggestion.command);
            println!();
            println!("  $ {}   [{}]", suggestion.command, describe(&verdict));

            let mut note = format!("suggested in {} post", suggestion.posts.len());
            if suggestion.posts.len() != 1 {
                note.push('s');
            }
            if suggestion.has_placeholder {
                note.push_str(", fill in the <...> part yourself");
            } else if suggestion.adapted {
                note.push_str(", adapted to your error");
            }
            println!("      {}, e.g. {}", note, suggestion.posts[0]);
            shown += 1;
        }
    }
    if hidden > 0 {
        println!();
        println!("  {} dangerous command(s) from these posts were hidden.", hidden);
    }

    if !strong {
        println!();
        if prediction.confidence < 0.5 && matches.is_empty() {
            println!("This doesn't look like an error ErrorObserver knows. Try pasting the exact error message.");
        } else {
            println!("General fixes for {}:", prediction.category);
            print_general_fixes(&rules, &safety, &prediction.category);
        }
        if let Some(url) = &safest_post {
            println!();
            println!("The most similar post may still help: {}", url);
        }
    }

    if let Some(command) = translator.translate(&input) {
        let verdict = safety.check(&command);
        println!();
        if verdict.level == Level::Block {
            println!("Suggested command hidden ({})", describe(&verdict));
        } else {
            println!("Suggested command: {}   [{}]", command, describe(&verdict));
        }
    }
}