//! Address parsing and pattern matching
//!
//! SignalFlow addresses follow this format:
//! ```text
//! /namespace/category/instance/property
//! /lumen/scene/0/layer/3/opacity
//! /midi/launchpad/cc/74
//! ```
//!
//! Wildcards (for subscriptions):
//! - `*` matches one segment
//! - `**` matches any number of segments

use crate::{Error, Result};

/// A parsed SignalFlow address
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    raw: String,
    segments: Vec<String>,
}

impl Address {
    /// Parse an address string
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(Error::InvalidAddress("empty address".to_string()));
        }

        if !s.starts_with('/') {
            return Err(Error::InvalidAddress(format!(
                "address must start with '/': {}",
                s
            )));
        }

        let segments: Vec<String> = s[1..]
            .split('/')
            .map(|s| s.to_string())
            .collect();

        // Validate segments
        for (i, seg) in segments.iter().enumerate() {
            if seg.is_empty() && i < segments.len() - 1 {
                return Err(Error::InvalidAddress(format!(
                    "empty segment in address: {}",
                    s
                )));
            }
        }

        Ok(Self {
            raw: s.to_string(),
            segments,
        })
    }

    /// Get the raw address string
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Get the address segments
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Get the namespace (first segment)
    pub fn namespace(&self) -> Option<&str> {
        self.segments.first().map(|s| s.as_str())
    }

    /// Get the last segment (usually the property name)
    pub fn property(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Check if this address contains wildcards
    pub fn is_pattern(&self) -> bool {
        self.segments.iter().any(|s| s == "*" || s == "**")
    }

    /// Check if this address matches a pattern
    pub fn matches(&self, pattern: &Address) -> bool {
        match_segments(&self.segments, &pattern.segments)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl TryFrom<&str> for Address {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        Address::parse(s)
    }
}

impl TryFrom<String> for Address {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Address::parse(&s)
    }
}

/// Match address segments against pattern segments
fn match_segments(addr: &[String], pattern: &[String]) -> bool {
    let mut ai = 0;
    let mut pi = 0;

    while pi < pattern.len() {
        let pat = &pattern[pi];

        if pat == "**" {
            // ** matches zero or more segments
            if pi == pattern.len() - 1 {
                // ** at end matches everything
                return true;
            }

            // Try to match remaining pattern after **
            let next_pat = &pattern[pi + 1];
            while ai < addr.len() {
                if match_single(&addr[ai], next_pat) {
                    // Try matching rest of pattern
                    if match_segments(&addr[ai..], &pattern[pi + 1..]) {
                        return true;
                    }
                }
                ai += 1;
            }
            return false;
        } else if ai >= addr.len() {
            return false;
        } else if !match_single(&addr[ai], pat) {
            return false;
        }

        ai += 1;
        pi += 1;
    }

    ai == addr.len()
}

/// Match a single segment against a pattern segment
fn match_single(segment: &str, pattern: &str) -> bool {
    if pattern == "*" {
        true
    } else {
        segment == pattern
    }
}

/// A compiled pattern for efficient matching
#[derive(Debug, Clone)]
pub struct Pattern {
    address: Address,
    regex: Option<regex_lite::Regex>,
}

impl Pattern {
    /// Compile a pattern from an address string
    pub fn compile(s: &str) -> Result<Self> {
        let address = Address::parse(s)?;

        // Build regex for efficient matching
        let regex = if address.is_pattern() {
            let regex_str = s
                .replace("**", "§§") // Temp placeholder
                .replace('*', "[^/]+")
                .replace("§§", ".*");
            let regex_str = format!("^{}$", regex_str);
            Some(
                regex_lite::Regex::new(&regex_str)
                    .map_err(|e| Error::InvalidPattern(e.to_string()))?,
            )
        } else {
            None
        };

        Ok(Self { address, regex })
    }

    /// Check if an address matches this pattern
    pub fn matches(&self, addr: &str) -> bool {
        if let Some(regex) = &self.regex {
            regex.is_match(addr)
        } else {
            addr == self.address.as_str()
        }
    }

    /// Check if an Address matches this pattern
    pub fn matches_address(&self, addr: &Address) -> bool {
        self.matches(addr.as_str())
    }

    /// Get the underlying address
    pub fn address(&self) -> &Address {
        &self.address
    }
}

// Use glob-match for simple cases
pub fn glob_match(pattern: &str, address: &str) -> bool {
    glob_match::glob_match(pattern, address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let addr = Address::parse("/lumen/scene/0/layer/3/opacity").unwrap();
        assert_eq!(addr.segments().len(), 6);
        assert_eq!(addr.namespace(), Some("lumen"));
        assert_eq!(addr.property(), Some("opacity"));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Address::parse("").is_err());
        assert!(Address::parse("no/leading/slash").is_err());
    }

    #[test]
    fn test_single_wildcard() {
        let pattern = Pattern::compile("/lumen/scene/*/layer/*/opacity").unwrap();

        assert!(pattern.matches("/lumen/scene/0/layer/3/opacity"));
        assert!(pattern.matches("/lumen/scene/1/layer/0/opacity"));
        assert!(!pattern.matches("/lumen/scene/0/layer/3/color"));
        assert!(!pattern.matches("/lumen/scene/opacity"));
    }

    #[test]
    fn test_double_wildcard() {
        let pattern = Pattern::compile("/lumen/**/opacity").unwrap();

        assert!(pattern.matches("/lumen/scene/0/opacity"));
        assert!(pattern.matches("/lumen/scene/0/layer/3/opacity"));
        assert!(pattern.matches("/lumen/opacity"));
        assert!(!pattern.matches("/lumen/scene/0/color"));
    }

    #[test]
    fn test_exact_match() {
        let pattern = Pattern::compile("/lumen/scene/0/opacity").unwrap();

        assert!(pattern.matches("/lumen/scene/0/opacity"));
        assert!(!pattern.matches("/lumen/scene/1/opacity"));
    }

    #[test]
    fn test_glob_match_fn() {
        assert!(glob_match("/lumen/**", "/lumen/scene/0/opacity"));
        assert!(glob_match("/lumen/*/opacity", "/lumen/scene/opacity"));
        assert!(!glob_match("/lumen/*/opacity", "/lumen/scene/0/opacity"));
    }
}
