pub struct IgnoreRules {
    patterns: Vec<String>,
}

impl IgnoreRules {
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    pub fn default_patterns() -> Vec<String> {
        vec![
            "*.swp".into(),
            "*.swx".into(),
            "*.tmp".into(),
            "*.temp".into(),
            "*~".into(),
            ".DS_Store".into(),
            "Thumbs.db".into(),
            ".~*".into(),
            "*.part".into(),
            ".goutputstream-*".into(),
        ]
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        self.patterns.iter().any(|p| matches_glob(path, p))
    }
}

impl Default for IgnoreRules {
    fn default() -> Self {
        Self::new(Self::default_patterns())
    }
}

fn matches_glob(path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }

    let filename = path.rsplit('/').next().unwrap_or(path);

    let parts: Vec<&str> = pattern.split('*').collect();

    match parts.len() {
        1 => filename == parts[0],
        2 => {
            let (prefix, suffix) = (parts[0], parts[1]);
            if prefix.is_empty() && suffix.is_empty() {
                true
            } else if prefix.is_empty() {
                filename.ends_with(suffix)
            } else if suffix.is_empty() {
                filename.starts_with(prefix)
            } else {
                filename.starts_with(prefix)
                    && filename.ends_with(suffix)
                    && filename.len() >= prefix.len() + suffix.len()
            }
        }
        _ => {
            let prefix = parts[0];
            let suffix = parts.last().unwrap_or(&"");
            filename.starts_with(prefix) && filename.ends_with(suffix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_glob_endswith() {
        assert!(matches_glob("f.swp", "*.swp"));
    }

    #[test]
    fn test_internal_glob_startswith() {
        assert!(matches_glob(".goutputstream-X", ".goutputstream-*"));
    }

    #[test]
    fn test_internal_glob_tilde_suffix() {
        assert!(matches_glob("f.txt~", "*~"));
    }

    #[test]
    fn test_internal_glob_exact() {
        assert!(matches_glob(".DS_Store", ".DS_Store"));
    }

    #[test]
    fn test_internal_empty_pattern() {
        assert!(!matches_glob("anything", ""));
    }
}
