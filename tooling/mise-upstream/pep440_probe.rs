use pep440_rs::{Version, VersionSpecifiers};
use std::str::FromStr;
fn main() {
    let cases = [
        (">=3.10,!=3.11.*,<3.14", "3.12.13", true),
        (">=3.10,!=3.11.*,<3.14", "3.11.9", false),
        (">=3.10,!=3.11.*,<3.14", "3.14.0", false),
        ("~=3.12", "3.13.5", true),
        ("~=3.12", "4.0.0", false),
        ("~=3.12.0", "3.13.0", false),
        ("~=3.12.0", "3.12.13", true),
        ("==3.12.*", "3.12.13", true),
        ("==3.12", "3.12.0", true),
        ("!=3.12.13", "3.12.13", false),
        (">3.12.0", "3.12.0.post1", false),
        ("==3.12.0", "3.12.0+local", true),
        (">=1!3.0", "3.99.0", false),
        ("===3.12.13", "3.12.13", true),
        (">=3.13.0a1", "3.13.0a2", true),
    ];
    for (query, candidate, expected) in cases {
        let spec = VersionSpecifiers::from_str(query).unwrap();
        let version = Version::from_str(candidate).unwrap();
        assert_eq!(spec.contains(&version), expected, "{query} / {candidate}");
    }
    for malformed in [
        "^3.12",
        "~3.12",
        ">=3.10 || <3.9",
        "~=3",
        ">=3.12.*",
        "nonsense",
    ] {
        assert!(
            VersionSpecifiers::from_str(malformed).is_err(),
            "{malformed}"
        );
    }
    println!("15 matching cases and 6 malformed specifiers passed");
}
