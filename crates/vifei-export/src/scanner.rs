//! Secret scanner for share-safe export.
//!
//! # Overview
//!
//! Scans event payloads and blob contents for secrets using pattern matching.
//! Conservative by design — false positives are safer than false negatives.
//!
//! # Pattern categories
//!
//! - **API keys**: AWS, OpenAI, Anthropic, generic formats
//! - **Tokens**: JWT, Bearer, OAuth
//! - **Secrets**: password=, secret=, api_key=, private keys
//! - **PII**: Email addresses, phone numbers (basic)
//!
//! # Usage
//!
//! ```ignore
//! let patterns = SecretPatterns::default();
//! let findings = scan_text(&patterns, "event:123", "payload", "AKIAIOSFODNN7EXAMPLE");
//! ```

use once_cell::sync::Lazy;
use regex::Regex;

/// A secret pattern with its detection regex and metadata.
#[derive(Debug, Clone)]
pub struct SecretPattern {
    /// Human-readable name for the pattern.
    pub name: &'static str,
    /// Category of secret (api_key, token, secret, pii).
    #[allow(dead_code)]
    pub category: &'static str,
    /// Compiled regex for detection.
    pub regex: &'static Lazy<Regex>,
}

/// A match found by the scanner.
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// Pattern name that matched.
    pub pattern_name: String,
    /// The matched text (will be redacted for output).
    pub matched_text: String,
    /// Byte offset in the scanned content.
    #[allow(dead_code)]
    pub offset: usize,
}

// ---------------------------------------------------------------------------
// Pattern definitions
// ---------------------------------------------------------------------------

// AWS Access Key ID: AKIA followed by 16 alphanumeric chars
static AWS_ACCESS_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"AKIA[0-9A-Z]{16}").expect("invalid regex"));

// AWS Secret Access Key: 40 character base64-like string
static AWS_SECRET_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)aws_secret_access_key\s*[=:]\s*[A-Za-z0-9/+=]{40}").expect("invalid regex")
});

// OpenAI API Key: sk- followed by alphanumeric (48 chars total)
static OPENAI_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-[A-Za-z0-9]{48}").expect("invalid regex"));

// Anthropic API Key: sk-ant- prefix
static ANTHROPIC_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-ant-[A-Za-z0-9_-]{90,}").expect("invalid regex"));

// Generic API key patterns
static GENERIC_API_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[=:]\s*['"]?[A-Za-z0-9_-]{20,}['"]?"#)
        .expect("invalid regex")
});

// JWT tokens: eyJ followed by base64url encoded data
static JWT_TOKEN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").expect("invalid regex")
});

// Bearer tokens
static BEARER_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)bearer\s+[A-Za-z0-9_-]{20,}").expect("invalid regex"));

// Password patterns
static PASSWORD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(password|passwd|pwd)\s*[=:]\s*['"]?[^\s'"]{8,}['"]?"#)
        .expect("invalid regex")
});

// Secret patterns
static SECRET_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)secret\s*[=:]\s*['"]?[A-Za-z0-9_/+=.-]{16,}['"]?"#).expect("invalid regex")
});

// Private key headers
static PRIVATE_KEY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-----BEGIN\s+(RSA|EC|DSA|OPENSSH|PGP)?\s*PRIVATE KEY-----").expect("invalid regex")
});

// GitHub personal access token
static GITHUB_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ghp_[A-Za-z0-9]{36}").expect("invalid regex"));

// Basic email pattern (not comprehensive, but catches obvious cases)
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").expect("invalid regex")
});

// Basic phone pattern (US format and international)
static PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:\+1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)?\d{3}[-.\s]?\d{4}").expect("invalid regex")
});

/// Collection of all secret patterns to scan for.
pub struct SecretPatterns {
    patterns: Vec<SecretPattern>,
}

impl Default for SecretPatterns {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretPatterns {
    /// Create a new pattern set with all default patterns.
    pub fn new() -> Self {
        SecretPatterns {
            patterns: vec![
                // API Keys
                SecretPattern {
                    name: "aws_access_key",
                    category: "api_key",
                    regex: &AWS_ACCESS_KEY,
                },
                SecretPattern {
                    name: "aws_secret_key",
                    category: "api_key",
                    regex: &AWS_SECRET_KEY,
                },
                SecretPattern {
                    name: "openai_key",
                    category: "api_key",
                    regex: &OPENAI_KEY,
                },
                SecretPattern {
                    name: "anthropic_key",
                    category: "api_key",
                    regex: &ANTHROPIC_KEY,
                },
                SecretPattern {
                    name: "generic_api_key",
                    category: "api_key",
                    regex: &GENERIC_API_KEY,
                },
                SecretPattern {
                    name: "github_token",
                    category: "api_key",
                    regex: &GITHUB_TOKEN,
                },
                // Tokens
                SecretPattern {
                    name: "jwt_token",
                    category: "token",
                    regex: &JWT_TOKEN,
                },
                SecretPattern {
                    name: "bearer_token",
                    category: "token",
                    regex: &BEARER_TOKEN,
                },
                // Secrets
                SecretPattern {
                    name: "password",
                    category: "secret",
                    regex: &PASSWORD_PATTERN,
                },
                SecretPattern {
                    name: "secret",
                    category: "secret",
                    regex: &SECRET_PATTERN,
                },
                SecretPattern {
                    name: "private_key",
                    category: "secret",
                    regex: &PRIVATE_KEY,
                },
                // PII
                SecretPattern {
                    name: "email",
                    category: "pii",
                    regex: &EMAIL_PATTERN,
                },
                SecretPattern {
                    name: "phone",
                    category: "pii",
                    regex: &PHONE_PATTERN,
                },
            ],
        }
    }

    /// Get all patterns.
    pub fn patterns(&self) -> &[SecretPattern] {
        &self.patterns
    }
}

/// Scan text content for secrets.
///
/// Returns all matches found in the content.
pub fn scan_text(patterns: &SecretPatterns, content: &str) -> Vec<SecretMatch> {
    let mut matches = Vec::new();

    for pattern in patterns.patterns() {
        for m in pattern.regex.find_iter(content) {
            matches.push(SecretMatch {
                pattern_name: pattern.name.to_string(),
                matched_text: m.as_str().to_string(),
                offset: m.start(),
            });
        }
    }

    matches
}

/// Scan binary content for secrets (treats as UTF-8 lossy).
///
/// For binary blobs, we do lossy UTF-8 conversion and scan the result.
/// This catches secrets embedded in text-like regions of binary data.
pub fn scan_bytes(patterns: &SecretPatterns, content: &[u8]) -> Vec<SecretMatch> {
    let text = String::from_utf8_lossy(content);
    scan_text(patterns, &text)
}

/// Redact a matched secret for safe display.
///
/// Shows first and last few characters with asterisks in between.
pub fn redact_match(matched: &str) -> String {
    let len = matched.len();
    if len <= 8 {
        "*".repeat(len)
    } else {
        let prefix = &matched[..4];
        let suffix = &matched[len - 4..];
        format!("{}***{}", prefix, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_access_key() {
        let patterns = SecretPatterns::new();
        let content = "my key is AKIAIOSFODNN7EXAMPLE in the config";
        let matches = scan_text(&patterns, content);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern_name, "aws_access_key");
        assert!(matches[0].matched_text.starts_with("AKIA"));
    }

    #[test]
    fn test_openai_key() {
        let patterns = SecretPatterns::new();
        // Exactly 48 alphanumeric chars after sk-
        let content = "export OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz1234567890123456789012";
        let matches = scan_text(&patterns, content);

        let openai_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "openai_key")
            .collect();
        assert_eq!(openai_matches.len(), 1);
    }

    #[test]
    fn test_jwt_token() {
        let patterns = SecretPatterns::new();
        let content = "token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let matches = scan_text(&patterns, content);

        let jwt_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "jwt_token")
            .collect();
        assert_eq!(jwt_matches.len(), 1);
    }

    #[test]
    fn test_password_pattern() {
        let patterns = SecretPatterns::new();
        let content = "password=mysecretpassword123";
        let matches = scan_text(&patterns, content);

        let pwd_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "password")
            .collect();
        assert_eq!(pwd_matches.len(), 1);
    }

    #[test]
    fn test_private_key() {
        let patterns = SecretPatterns::new();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let matches = scan_text(&patterns, content);

        let key_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "private_key")
            .collect();
        assert_eq!(key_matches.len(), 1);
    }

    #[test]
    fn test_email_pattern() {
        let patterns = SecretPatterns::new();
        let content = "contact me at user@example.com for details";
        let matches = scan_text(&patterns, content);

        let email_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "email")
            .collect();
        assert_eq!(email_matches.len(), 1);
        assert_eq!(email_matches[0].matched_text, "user@example.com");
    }

    #[test]
    fn test_github_token() {
        let patterns = SecretPatterns::new();
        // Exactly 36 alphanumeric chars after ghp_
        let content = "GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz1234567890";
        let matches = scan_text(&patterns, content);

        let gh_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "github_token")
            .collect();
        assert_eq!(gh_matches.len(), 1);
    }

    #[test]
    fn test_clean_content() {
        let patterns = SecretPatterns::new();
        let content = "This is just regular text with no secrets.";
        let matches = scan_text(&patterns, content);

        // Filter out PII patterns which might match regular text
        let secret_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name != "email" && m.pattern_name != "phone")
            .collect();
        assert!(secret_matches.is_empty());
    }

    #[test]
    fn test_redact_short() {
        assert_eq!(redact_match("secret"), "******");
    }

    #[test]
    fn test_redact_long() {
        let redacted = redact_match("AKIAIOSFODNN7EXAMPLE");
        assert!(redacted.starts_with("AKIA"));
        assert!(redacted.ends_with("MPLE"));
        assert!(redacted.contains("***"));
    }

    #[test]
    fn test_scan_bytes() {
        let patterns = SecretPatterns::new();
        let content = b"api_key=abcdefghij1234567890klmnopqrstuvwxyz";
        let matches = scan_bytes(&patterns, content);

        assert!(!matches.is_empty());
    }

    #[test]
    fn test_multiple_matches() {
        let patterns = SecretPatterns::new();
        let content = "AKIAIOSFODNN7EXAMPLE and password=secret123456789";
        let matches = scan_text(&patterns, content);

        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_bearer_token() {
        let patterns = SecretPatterns::new();
        let content = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let matches = scan_text(&patterns, content);

        let bearer_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.pattern_name == "bearer_token")
            .collect();
        assert_eq!(bearer_matches.len(), 1);
    }
}
