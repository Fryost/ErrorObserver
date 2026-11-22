use regex::Regex;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Level {
    Safe,
    Warn,
    Block,
}

pub struct Verdict {
    pub level: Level,
    pub reason: String,
}

struct Rule {
    pattern: Regex,
    level: Level,
    reason: String,
}

pub struct SafetyChecker {
    rules: Vec<Rule>,
}

impl SafetyChecker {
    pub fn new() -> SafetyChecker {
        let mut checker = SafetyChecker { rules: Vec::new() };

        checker.add(Level::Block, r"\brm\s+(-[\w-]*\s+)*-\w*[rR]\w*\s+(-[\w-]+\s+)*(/|/\*|~|\$HOME)(\s|$)", "deletes the whole system or home folder");
        checker.add(Level::Block, r"\bmkfs(\.\w+)?\b", "formats a disk");
        checker.add(Level::Block, r"\bdd\b.*\bof=/dev/", "overwrites a disk");
        checker.add(Level::Block, r">\s*/dev/(sd|nvme|hd|vd)", "overwrites a disk");
        checker.add(Level::Block, r":\(\)\s*\{\s*:\|:&\s*\};:", "fork bomb");
        checker.add(Level::Block, r"\bchmod\s+(-[\w-]+\s+)*-\w*R\w*\s+777\s+/(\s|$)", "makes every file writable by everyone");
        checker.add(Level::Block, r"\b(curl|wget)\b.*\|\s*(sudo\s+)?(ba|z)?sh\b", "runs a script from the internet unchecked");

        checker.add(Level::Warn, r"\brm\b", "deletes files permanently");
        checker.add(Level::Warn, r"\b(apt|apt-get|dpkg)\b.*\s(remove|purge|autoremove|-r|-P)\b", "uninstalls software");
        checker.add(Level::Warn, r"\b(shutdown|reboot|poweroff)\b", "restarts or turns off the computer");
        checker.add(Level::Warn, r"\bchmod\s+(-\w+\s+)*777\b", "makes files writable by everyone");
        checker.add(Level::Warn, r"\bsudo\b", "runs with administrator rights");

        checker
    }

    fn add(&mut self, level: Level, pattern: &str, reason: &str) {
        let rule = Rule {
            pattern: Regex::new(pattern).unwrap(),
            level,
            reason: reason.to_string(),
        };
        self.rules.push(rule);
    }

    pub fn check(&self, command: &str) -> Verdict {
        for rule in &self.rules {
            if rule.pattern.is_match(command) {
                return Verdict { level: rule.level, reason: rule.reason.clone() };
            }
        }
        Verdict { level: Level::Safe, reason: String::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_dangerous() {
        let checker = SafetyChecker::new();
        let commands = ["rm -rf /", "sudo rm -rf --no-preserve-root /", "rm -fr ~", "mkfs.ext4 /dev/sda1", "dd if=/dev/zero of=/dev/sda", ":(){ :|:& };:", "curl http://x.sh | sudo bash", "chmod -R 777 /"];
        for command in commands {
            assert!(checker.check(command).level == Level::Block, "{}", command);
        }
    }

    #[test]
    fn warns_risky() {
        let checker = SafetyChecker::new();
        let commands = ["rm notes.txt", "apt remove openssh-server", "sudo apt update", "chmod 777 file", "reboot"];
        for command in commands {
            assert!(checker.check(command).level == Level::Warn, "{}", command);
        }
    }

    #[test]
    fn allows_safe() {
        let checker = SafetyChecker::new();
        let commands = ["ls -l", "df -h", "ping -c 3 1.1.1.1", "rm -rf ./build"];
        for command in commands {
            assert!(checker.check(command).level != Level::Block, "{}", command);
        }
    }
}