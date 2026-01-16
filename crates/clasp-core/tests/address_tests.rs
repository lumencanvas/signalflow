//! Address parsing and matching tests

use clasp_core::Address;

#[test]
fn test_address_parse() {
    let addr = Address::parse("/lumen/layer/0/opacity").unwrap();
    assert_eq!(addr.as_str(), "/lumen/layer/0/opacity");
    assert_eq!(addr.segments().len(), 4);
    assert_eq!(addr.segments(), &["lumen", "layer", "0", "opacity"]);
}

#[test]
fn test_address_empty() {
    let result = Address::parse("");
    assert!(result.is_err());
}

#[test]
fn test_address_no_leading_slash() {
    let result = Address::parse("no/leading/slash");
    assert!(result.is_err());
}

#[test]
fn test_address_root() {
    let addr = Address::parse("/").unwrap();
    assert_eq!(addr.as_str(), "/");
    // Root address has one empty segment from splitting
    assert_eq!(addr.segments().len(), 1);
}

#[test]
fn test_address_single_segment() {
    let addr = Address::parse("/single").unwrap();
    assert_eq!(addr.segments(), &["single"]);
}

#[test]
fn test_pattern_match_exact() {
    let pattern = Address::parse("/lumen/layer/0/opacity").unwrap();
    let address = Address::parse("/lumen/layer/0/opacity").unwrap();

    assert!(address.matches(&pattern));
}

#[test]
fn test_pattern_match_single_wildcard() {
    let pattern = Address::parse("/lumen/layer/*/opacity").unwrap();

    let match1 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match2 = Address::parse("/lumen/layer/5/opacity").unwrap();
    let match3 = Address::parse("/lumen/layer/foo/opacity").unwrap();

    assert!(match1.matches(&pattern));
    assert!(match2.matches(&pattern));
    assert!(match3.matches(&pattern));

    let no_match1 = Address::parse("/lumen/layer/opacity").unwrap();
    let no_match2 = Address::parse("/lumen/layer/0/0/opacity").unwrap();
    let no_match3 = Address::parse("/other/layer/0/opacity").unwrap();

    assert!(!no_match1.matches(&pattern));
    assert!(!no_match2.matches(&pattern));
    assert!(!no_match3.matches(&pattern));
}

#[test]
fn test_pattern_match_multi_wildcard() {
    let pattern = Address::parse("/lumen/**/opacity").unwrap();

    let match1 = Address::parse("/lumen/opacity").unwrap();
    let match2 = Address::parse("/lumen/layer/opacity").unwrap();
    let match3 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match4 = Address::parse("/lumen/a/b/c/d/opacity").unwrap();

    assert!(match1.matches(&pattern));
    assert!(match2.matches(&pattern));
    assert!(match3.matches(&pattern));
    assert!(match4.matches(&pattern));

    let no_match1 = Address::parse("/lumen/layer/0/enabled").unwrap();
    let no_match2 = Address::parse("/other/opacity").unwrap();

    assert!(!no_match1.matches(&pattern));
    assert!(!no_match2.matches(&pattern));
}

#[test]
fn test_pattern_match_multiple_single_wildcards() {
    let pattern = Address::parse("/*/layer/*/opacity").unwrap();

    let match1 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match2 = Address::parse("/foo/layer/bar/opacity").unwrap();

    assert!(match1.matches(&pattern));
    assert!(match2.matches(&pattern));

    let no_match = Address::parse("/lumen/group/0/opacity").unwrap();
    assert!(!no_match.matches(&pattern));
}

#[test]
fn test_pattern_match_trailing_multi_wildcard() {
    let pattern = Address::parse("/lumen/**").unwrap();

    let match1 = Address::parse("/lumen").unwrap();
    let match2 = Address::parse("/lumen/layer").unwrap();
    let match3 = Address::parse("/lumen/layer/0/opacity").unwrap();

    assert!(match1.matches(&pattern));
    assert!(match2.matches(&pattern));
    assert!(match3.matches(&pattern));

    let no_match = Address::parse("/other/something").unwrap();
    assert!(!no_match.matches(&pattern));
}

#[test]
fn test_address_namespace() {
    let addr = Address::parse("/lumen/layer/0/opacity").unwrap();
    assert_eq!(addr.namespace(), Some("lumen"));
}

#[test]
fn test_address_property() {
    let addr = Address::parse("/lumen/layer/0/opacity").unwrap();
    assert_eq!(addr.property(), Some("opacity"));
}

#[test]
fn test_address_is_pattern() {
    let addr = Address::parse("/lumen/layer/0").unwrap();
    assert!(!addr.is_pattern());

    let pattern1 = Address::parse("/lumen/*/opacity").unwrap();
    assert!(pattern1.is_pattern());

    let pattern2 = Address::parse("/lumen/**").unwrap();
    assert!(pattern2.is_pattern());
}
