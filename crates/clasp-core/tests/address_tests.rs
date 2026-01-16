//! Address parsing and matching tests

use clasp_core::Address;

#[test]
fn test_address_parse() {
    let addr = Address::parse("/lumen/layer/0/opacity").unwrap();
    assert_eq!(addr.path(), "/lumen/layer/0/opacity");
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
    assert_eq!(addr.path(), "/");
    assert!(addr.segments().is_empty());
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

    assert!(pattern.matches(&address));
}

#[test]
fn test_pattern_match_single_wildcard() {
    let pattern = Address::parse("/lumen/layer/*/opacity").unwrap();

    let match1 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match2 = Address::parse("/lumen/layer/5/opacity").unwrap();
    let match3 = Address::parse("/lumen/layer/foo/opacity").unwrap();

    assert!(pattern.matches(&match1));
    assert!(pattern.matches(&match2));
    assert!(pattern.matches(&match3));

    let no_match1 = Address::parse("/lumen/layer/opacity").unwrap();
    let no_match2 = Address::parse("/lumen/layer/0/0/opacity").unwrap();
    let no_match3 = Address::parse("/other/layer/0/opacity").unwrap();

    assert!(!pattern.matches(&no_match1));
    assert!(!pattern.matches(&no_match2));
    assert!(!pattern.matches(&no_match3));
}

#[test]
fn test_pattern_match_multi_wildcard() {
    let pattern = Address::parse("/lumen/**/opacity").unwrap();

    let match1 = Address::parse("/lumen/opacity").unwrap();
    let match2 = Address::parse("/lumen/layer/opacity").unwrap();
    let match3 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match4 = Address::parse("/lumen/a/b/c/d/opacity").unwrap();

    assert!(pattern.matches(&match1));
    assert!(pattern.matches(&match2));
    assert!(pattern.matches(&match3));
    assert!(pattern.matches(&match4));

    let no_match1 = Address::parse("/lumen/layer/0/enabled").unwrap();
    let no_match2 = Address::parse("/other/opacity").unwrap();

    assert!(!pattern.matches(&no_match1));
    assert!(!pattern.matches(&no_match2));
}

#[test]
fn test_pattern_match_multiple_single_wildcards() {
    let pattern = Address::parse("/*/layer/*/opacity").unwrap();

    let match1 = Address::parse("/lumen/layer/0/opacity").unwrap();
    let match2 = Address::parse("/foo/layer/bar/opacity").unwrap();

    assert!(pattern.matches(&match1));
    assert!(pattern.matches(&match2));

    let no_match = Address::parse("/lumen/group/0/opacity").unwrap();
    assert!(!pattern.matches(&no_match));
}

#[test]
fn test_pattern_match_trailing_multi_wildcard() {
    let pattern = Address::parse("/lumen/**").unwrap();

    let match1 = Address::parse("/lumen").unwrap();
    let match2 = Address::parse("/lumen/layer").unwrap();
    let match3 = Address::parse("/lumen/layer/0/opacity").unwrap();

    assert!(pattern.matches(&match1));
    assert!(pattern.matches(&match2));
    assert!(pattern.matches(&match3));

    let no_match = Address::parse("/other/something").unwrap();
    assert!(!pattern.matches(&no_match));
}

#[test]
fn test_address_parent() {
    let addr = Address::parse("/lumen/layer/0/opacity").unwrap();
    let parent = addr.parent().unwrap();

    assert_eq!(parent.path(), "/lumen/layer/0");
}

#[test]
fn test_address_parent_root() {
    let addr = Address::parse("/single").unwrap();
    let parent = addr.parent().unwrap();

    assert_eq!(parent.path(), "/");
}

#[test]
fn test_address_join() {
    let base = Address::parse("/lumen/layer").unwrap();
    let joined = base.join("0").unwrap();

    assert_eq!(joined.path(), "/lumen/layer/0");
}
