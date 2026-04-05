use regex::Regex;
use std::collections::HashMap;
use tracing::{info, warn};
use lazy_static::lazy_static;

lazy_static! {
    // Common secret patterns to detect
    static ref SECRET_PATTERNS: Vec<(String, Regex)> = vec![
        ("AWS Access Key".to_string(), Regex::new(r"A(?:3T[A-Z0-9]|KIA)[A-Z0-9]{16}").unwrap()),
        ("AWS Secret Key".to_string(), Regex::new(r"(?i)aws[_\-]?secret[_\-]?.{0,20}[\x22\x27]?[A-Za-z0-9/+=]{40}[\x22\x27]?").unwrap()),
        ("Private Key".to_string(), Regex::new(r"-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----").unwrap()),
        ("Generic API Key".to_string(), Regex::new(r"(?i)(?:api[_\-]?key|apikey)[\x22\x27]?\s*[:=]\s*[\x22\x27]?[A-Za-z0-9]{20,}[\x22\x27]?").unwrap()),
        ("JWT Token".to_string(), Regex::new(r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap()),
        ("GitHub Token".to_string(), Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap()),
        ("Slack Webhook".to_string(), Regex::new(r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+").unwrap()),
    ];
}

pub struct LeakDetector {
    honeytokens: HashMap<String, String>, // Fake secrets to detect leaks
}

impl LeakDetector {
    pub fn new() -> Self {
        Self {
            honeytokens: HashMap::new(),
        }
    }

    /// Register a honeytoken (fake secret) to detect if it appears in logs/code
    pub fn register_honeytoken(&mut self, name: &str, value: &str) {
        self.honeytokens.insert(name.to_string(), value.to_string());
        info!("🍯 Registered honeytoken: {}", name);
    }

    /// Scan text for potential secret leaks
    pub fn scan_for_leaks(&self, text: &str) -> Vec<String> {
        let mut findings = Vec::new();

        // Check against known patterns
        for (pattern_name, regex) in SECRET_PATTERNS.iter() {
            if regex.is_match(text) {
                findings.push(format!("Potential {} detected", pattern_name));
                warn!("⚠️  {}", findings.last().unwrap());
            }
        }

        // Check for honeytoken leaks
        for (name, token) in &self.honeytokens {
            if text.contains(token) {
                let msg = format!("🚨 HONEYTOKEN LEAK DETECTED: {}", name);
                findings.push(msg.clone());
                warn!("{}", msg);
            }
        }

        findings
    }

    /// Scan environment variables for potential leaks
    pub fn scan_environment(&self) -> Vec<String> {
        let mut all_findings = Vec::new();

        for (key, value) in std::env::vars() {
            let findings = self.scan_for_leaks(&value);
            for finding in findings {
                all_findings.push(format!("Env var {}: {}", key, finding));
            }
        }

        all_findings
    }

    /// Validate that secrets are not being logged
    pub fn validate_no_secrets_in_logs(&self, log_content: &str) -> bool {
        let findings = self.scan_for_leaks(log_content);
        findings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_aws_key() {
        let detector = LeakDetector::new();
        let text = "My AWS key is AKIAIOSFODNN7EXAMPLE";
        let findings = detector.scan_for_leaks(text);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_detects_private_key() {
        let detector = LeakDetector::new();
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let findings = detector.scan_for_leaks(text);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_honeytoken_detection() {
        let mut detector = LeakDetector::new();
        detector.register_honeytoken("test_token", "FAKE_SECRET_12345");
        
        let text = "Oops, I leaked FAKE_SECRET_12345 in my code";
        let findings = detector.scan_for_leaks(text);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.contains("HONEYTOKEN")));
    }
}
