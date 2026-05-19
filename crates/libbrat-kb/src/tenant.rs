//! Tenant mapping from repo path to zkb tenant name.

use std::path::Path;

/// Convert a repo path to a zkb tenant name.
///
/// Uses the repo directory name with a `brat-` prefix and slugification.
///
/// # Examples
///
/// ```
/// use libbrat_kb::tenant::repo_to_tenant;
/// use std::path::Path;
///
/// assert_eq!(repo_to_tenant(Path::new("/home/user/my-project")), "brat-my-project");
/// ```
pub fn repo_to_tenant(repo_root: &Path) -> String {
    let slug = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_lowercase()
        .replace(' ', "-")
        .replace('_', "-");

    format!("brat-{}", slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_repo_to_tenant() {
        assert_eq!(
            repo_to_tenant(Path::new("/home/user/my-project")),
            "brat-my-project"
        );
        assert_eq!(
            repo_to_tenant(Path::new("/home/user/my_project")),
            "brat-my-project"
        );
        assert_eq!(
            repo_to_tenant(Path::new("/home/user/My Project")),
            "brat-my-project"
        );
    }
}
