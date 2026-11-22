pub struct Adapted {
    pub command: String,
    pub changed: bool,
    pub has_placeholder: bool,
}

fn is_word_character(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '-' || c == '+' || c == '/' || c == '~'
}

fn split_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if is_word_character(c) {
            current.push(c);
        } else if !current.is_empty() {
            words.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    let mut cleaned = Vec::new();
    for word in words {
        let trimmed = word.trim_end_matches(|c| c == '.' || c == '-').to_string();
        if !trimmed.is_empty() {
            cleaned.push(trimmed);
        }
    }
    cleaned
}

fn common_positions(a: &Vec<String>, b: &Vec<String>) -> Vec<(usize, usize)> {
    let n = a.len();
    let m = b.len();
    let mut table = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            if a[i] == b[j] {
                table[i][j] = table[i + 1][j + 1] + 1;
            } else if table[i + 1][j] >= table[i][j + 1] {
                table[i][j] = table[i + 1][j];
            } else {
                table[i][j] = table[i][j + 1];
            }
        }
    }

    let mut pairs = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < n && j < m {
        if a[i] == b[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

pub struct Differences {
    pub swaps: Vec<(String, String)>,
    pub unmatched: Vec<String>,
}

pub fn compare(their_error: &str, your_error: &str) -> Differences {
    let theirs = split_words(their_error);
    let yours = split_words(your_error);
    let mut anchors = common_positions(&theirs, &yours);
    anchors.push((theirs.len(), yours.len()));

    let mut swaps = Vec::new();
    let mut unmatched = Vec::new();
    let mut start_theirs = 0;
    let mut start_yours = 0;

    for (end_theirs, end_yours) in anchors {
        let gap_theirs = &theirs[start_theirs..end_theirs];
        let gap_yours = &yours[start_yours..end_yours];
        if gap_theirs.len() == gap_yours.len() {
            for k in 0..gap_theirs.len() {
                swaps.push((gap_theirs[k].clone(), gap_yours[k].clone()));
            }
        } else {
            for word in gap_theirs {
                unmatched.push(word.clone());
            }
        }
        start_theirs = end_theirs + 1;
        start_yours = end_yours + 1;
    }

    Differences { swaps, unmatched }
}

fn is_separator(c: char) -> bool {
    c.is_whitespace() || c == '\'' || c == '"' || c == '=' || c == ':' || c == ',' || c == ';' || c == '(' || c == ')'
}

fn replace_word(text: &str, old: &str, new: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let old_characters: Vec<char> = old.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < characters.len() {
        let fits = i + old_characters.len() <= characters.len() && characters[i..i + old_characters.len()] == old_characters[..];
        let start_ok = i == 0 || is_separator(characters[i - 1]);
        let end = i + old_characters.len();
        let end_ok = end >= characters.len() || is_separator(characters[end]);
        if !old_characters.is_empty() && fits && start_ok && end_ok {
            result.push_str(new);
            i = end;
        } else {
            result.push(characters[i]);
            i += 1;
        }
    }
    result
}

fn looks_specific(word: &str) -> bool {
    for c in word.chars() {
        if c.is_ascii_digit() || c == '/' || c == '.' || c == '_' {
            return true;
        }
    }
    false
}

fn placeholder_for(word: &str) -> &'static str {
    if word.contains('/') {
        "<file>"
    } else {
        "<name>"
    }
}

pub fn adapt(command: &str, differences: &Differences) -> Adapted {
    let mut adapted = command.to_string();
    let mut has_placeholder = false;

    for (old, new) in &differences.swaps {
        if old.len() >= 2 {
            adapted = replace_word(&adapted, old, new);
        }
    }
    for word in &differences.unmatched {
        if word.len() >= 2 && looks_specific(word) {
            let replaced = replace_word(&adapted, word, placeholder_for(word));
            if replaced != adapted {
                has_placeholder = true;
                adapted = replaced;
            }
        }
    }

    let changed = adapted != command;
    Adapted { command: adapted, changed, has_placeholder }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swaps_package_name() {
        let differences = compare("E: Unable to locate package nginx18", "E: Unable to locate package htop");
        let result = adapt("sudo apt install nginx18", &differences);
        assert_eq!(result.command, "sudo apt install htop");
        assert!(result.changed);
    }

    #[test]
    fn swaps_path() {
        let differences = compare("bash: /etc/foo.conf: Permission denied", "bash: /var/log/syslog: Permission denied");
        let result = adapt("sudo chown $USER /etc/foo.conf", &differences);
        assert_eq!(result.command, "sudo chown $USER /var/log/syslog");
    }

    #[test]
    fn keeps_command_words() {
        let differences = compare("E: Unable to locate package nginx18", "E: Unable to locate package htop");
        let result = adapt("sudo apt update", &differences);
        assert_eq!(result.command, "sudo apt update");
        assert!(!result.changed);
    }

    #[test]
    fn uses_placeholder_when_unclear() {
        let differences = compare("cp: error writing '/home/a/b.iso': No space left on device", "write error: No space left on device");
        let result = adapt("rm /home/a/b.iso", &differences);
        assert_eq!(result.command, "rm <file>");
        assert!(result.has_placeholder);
    }

    #[test]
    fn does_not_change_part_of_a_name() {
        let differences = compare("E: Unable to locate package libpcap", "E: Unable to locate package htop");
        let result = adapt("sudo apt-get install libpcap-dev", &differences);
        assert_eq!(result.command, "sudo apt-get install libpcap-dev");
    }

    #[test]
    fn same_error_changes_nothing() {
        let differences = compare("Segmentation fault (core dumped)", "Segmentation fault (core dumped)");
        let result = adapt("gdb ./program core", &differences);
        assert!(!result.changed);
    }
}