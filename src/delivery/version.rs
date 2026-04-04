//! Version management — semver parsing and bumping

use crate::error::{Result, ZcodeError};

/// Type of version bump
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

impl std::fmt::Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Patch => write!(f, "patch"),
        }
    }
}

/// Version manager — handles semver parsing and bumping
pub struct VersionManager;

impl VersionManager {
    /// Detect the appropriate bump type from task descriptions
    /// Uses keyword matching to determine if this is a breaking, feature, or patch release
    pub fn detect_bump_type(task_descriptions: &[String]) -> BumpType {
        for desc in task_descriptions {
            let desc_lower = desc.to_lowercase();
            // Check for breaking changes first
            if desc_lower.contains("breaking")
                || desc_lower.contains("remove")
                || desc_lower.contains("deprecat")
            {
                return BumpType::Major;
            }
        }

        for desc in task_descriptions {
            let desc_lower = desc.to_lowercase();
            // Check for new features
            if desc_lower.contains("add")
                || desc_lower.contains("new")
                || desc_lower.contains("implement")
                || desc_lower.contains("feature")
            {
                return BumpType::Minor;
            }
        }

        // Default to patch for bug fixes and other changes
        BumpType::Patch
    }

    /// Bump a semantic version string
    /// Input format: "x.y.z" where x, y, z are non-negative integers
    /// Returns the new version string
    pub fn bump_version(version: &str, bump: BumpType) -> Result<String> {
        let (major, minor, patch) = Self::parse_version(version)?;

        let (new_major, new_minor, new_patch) = match bump {
            BumpType::Major => (major + 1, 0, 0),
            BumpType::Minor => (major, minor + 1, 0),
            BumpType::Patch => (major, minor, patch + 1),
        };

        Ok(format!("{}.{}.{}", new_major, new_minor, new_patch))
    }

    /// Parse a semver string into (major, minor, patch) components
    /// Input format: "x.y.z" where x, y, z are non-negative integers
    pub fn parse_version(version: &str) -> Result<(u32, u32, u32)> {
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() != 3 {
            return Err(ZcodeError::InternalError(format!(
                "Invalid version format '{}': expected x.y.z",
                version
            )));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| ZcodeError::InternalError(format!(
                "Invalid major version '{}' in '{}'",
                parts[0], version
            )))?;

        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| ZcodeError::InternalError(format!(
                "Invalid minor version '{}' in '{}'",
                parts[1], version
            )))?;

        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| ZcodeError::InternalError(format!(
                "Invalid patch version '{}' in '{}'",
                parts[2], version
            )))?;

        Ok((major, minor, patch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_type_display() {
        assert_eq!(format!("{}", BumpType::Major), "major");
        assert_eq!(format!("{}", BumpType::Minor), "minor");
        assert_eq!(format!("{}", BumpType::Patch), "patch");
    }

    #[test]
    fn test_detect_bump_type_breaking() {
        let tasks = vec![
            "Add new feature".to_string(),
            "Breaking change to API".to_string(),
            "Fix bug".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Major
        );
    }

    #[test]
    fn test_detect_bump_type_remove() {
        let tasks = vec![
            "Remove old endpoint".to_string(),
            "Fix bug".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Major
        );
    }

    #[test]
    fn test_detect_bump_type_deprecate() {
        let tasks = vec![
            "Deprecate v1 API".to_string(),
            "Add new feature".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Major
        );
    }

    #[test]
    fn test_detect_bump_type_feature() {
        let tasks = vec![
            "Add auth module".to_string(),
            "Fix bug".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Minor
        );
    }

    #[test]
    fn test_detect_bump_type_implement() {
        let tasks = vec![
            "Implement caching".to_string(),
            "Fix bug".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Minor
        );
    }

    #[test]
    fn test_detect_bump_type_new() {
        let tasks = vec![
            "New dashboard".to_string(),
            "Fix bug".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Minor
        );
    }

    #[test]
    fn test_detect_bump_type_patch_default() {
        let tasks = vec![
            "Fix login bug".to_string(),
            "Update documentation".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Patch
        );
    }

    #[test]
    fn test_detect_bump_type_empty() {
        let tasks: Vec<String> = vec![];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Patch
        );
    }

    #[test]
    fn test_detect_bump_type_case_insensitive() {
        let tasks = vec![
            "BREAKING change".to_string(),
            "Add feature".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Major
        );
    }

    #[test]
    fn test_parse_version_valid() {
        assert_eq!(VersionManager::parse_version("1.0.0").unwrap(), (1, 0, 0));
        assert_eq!(VersionManager::parse_version("0.1.0").unwrap(), (0, 1, 0));
        assert_eq!(VersionManager::parse_version("10.20.30").unwrap(), (10, 20, 30));
        assert_eq!(VersionManager::parse_version("0.0.0").unwrap(), (0, 0, 0));
    }

    #[test]
    fn test_parse_version_invalid_format() {
        assert!(VersionManager::parse_version("1.0").is_err());
        assert!(VersionManager::parse_version("1").is_err());
        assert!(VersionManager::parse_version("1.0.0.0").is_err());
        assert!(VersionManager::parse_version("").is_err());
    }

    #[test]
    fn test_parse_version_invalid_numbers() {
        assert!(VersionManager::parse_version("a.b.c").is_err());
        assert!(VersionManager::parse_version("1.2.x").is_err());
        assert!(VersionManager::parse_version("-1.0.0").is_err());
    }

    #[test]
    fn test_bump_version_major() {
        assert_eq!(VersionManager::bump_version("1.0.0", BumpType::Major).unwrap(), "2.0.0");
        assert_eq!(VersionManager::bump_version("0.1.0", BumpType::Major).unwrap(), "1.0.0");
        assert_eq!(VersionManager::bump_version("10.5.3", BumpType::Major).unwrap(), "11.0.0");
    }

    #[test]
    fn test_bump_version_minor() {
        assert_eq!(VersionManager::bump_version("1.0.0", BumpType::Minor).unwrap(), "1.1.0");
        assert_eq!(VersionManager::bump_version("1.5.0", BumpType::Minor).unwrap(), "1.6.0");
        assert_eq!(VersionManager::bump_version("0.0.0", BumpType::Minor).unwrap(), "0.1.0");
    }

    #[test]
    fn test_bump_version_patch() {
        assert_eq!(VersionManager::bump_version("1.0.0", BumpType::Patch).unwrap(), "1.0.1");
        assert_eq!(VersionManager::bump_version("1.5.10", BumpType::Patch).unwrap(), "1.5.11");
        assert_eq!(VersionManager::bump_version("0.0.0", BumpType::Patch).unwrap(), "0.0.1");
    }

    #[test]
    fn test_bump_version_preserves_major_minor() {
        assert_eq!(VersionManager::bump_version("5.10.3", BumpType::Patch).unwrap(), "5.10.4");
        assert_eq!(VersionManager::bump_version("5.10.3", BumpType::Minor).unwrap(), "5.11.0");
    }

    #[test]
    fn test_bump_version_invalid_input() {
        assert!(VersionManager::bump_version("invalid", BumpType::Patch).is_err());
        assert!(VersionManager::bump_version("1.0", BumpType::Patch).is_err());
    }

    #[test]
    fn test_roundtrip_bump_and_parse() {
        let version = "2.5.10";
        let (major, minor, patch) = VersionManager::parse_version(version).unwrap();
        assert_eq!((major, minor, patch), (2, 5, 10));

        let bumped = VersionManager::bump_version(version, BumpType::Minor).unwrap();
        assert_eq!(bumped, "2.6.0");

        let (new_major, new_minor, new_patch) = VersionManager::parse_version(&bumped).unwrap();
        assert_eq!((new_major, new_minor, new_patch), (2, 6, 0));
    }

    #[test]
    fn test_bump_type_equality() {
        assert_eq!(BumpType::Major, BumpType::Major);
        assert_eq!(BumpType::Minor, BumpType::Minor);
        assert_eq!(BumpType::Patch, BumpType::Patch);
        assert_ne!(BumpType::Major, BumpType::Minor);
    }

    #[test]
    fn test_multiple_breaking_keywords() {
        let tasks = vec![
            "Add feature".to_string(),
            "Remove old code".to_string(),
            "Deprecate API".to_string(),
        ];
        // Should return Major on first breaking match
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Major
        );
    }

    #[test]
    fn test_patch_only_keywords() {
        let tasks = vec![
            "Fix bug in parser".to_string(),
            "Patch security issue".to_string(),
            "Update docs".to_string(),
        ];
        assert_eq!(
            VersionManager::detect_bump_type(&tasks),
            BumpType::Patch
        );
    }
}
