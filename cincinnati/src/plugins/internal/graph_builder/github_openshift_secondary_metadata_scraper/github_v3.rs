//! This is a helper module for accessing the [GitHub API v3][].
//!
//! [GitHub API v3]: https://developer.github.com/v3/

use serde::Deserialize;

/// Commit structure.
#[derive(Default, Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) sha: String,
    pub(crate) url: String,
}

/// Branch structure.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Branch {
    pub(crate) name: String,
    pub(crate) commit: Commit,
    pub(crate) protected: bool,
}

/// Format the URL to request branch information.
pub(crate) fn branches_url(org: &str, repo: &str) -> String {
    format!(
        "https://api.github.com/repos/{org}/{repo}/branches",
        org = &org,
        repo = &repo,
    )
}

/// Format the URL to request a tarball URL.
pub(crate) fn tarball_url(org: &str, repo: &str, commit: &Commit) -> String {
    format!(
        "https://api.github.com/repos/{org}/{repo}/tarball/{sha}",
        org = org,
        repo = repo,
        sha = commit.sha,
    )
}

/// Format a subdirectory name for a specific revision's tarball.
pub(crate) fn archive_entry_directory_name(org: &str, repo: &str, commit: &Commit) -> String {
    format!("{}-{}-{}", &org, &repo, &commit.sha[0..7],)
}

/// Format a commit URL
pub(crate) fn commit_url(org: &str, repo: &str, sha: &str) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        org, repo, sha
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_serialize_branch() {
        let json = r#"
            [
                {
                    "name": "master",
                    "commit": {
                        "sha": "fef06adb57b9d965bfc9ae0959bd038f3044207e",
                        "url": "https://api.github.com/repos/openshift/cincinnati-graph-data/commits/fef06adb57b9d965bfc9ae0959bd038f3044207e"
                    },
                    "protected": true
                }
            ]
            "#;

        let branches = serde_json::from_str::<Vec<Branch>>(json).unwrap();

        let branches_expected = vec![Branch {
                name: "master".to_string(),
                commit: Commit {
                        sha: "fef06adb57b9d965bfc9ae0959bd038f3044207e".to_string(),
                        url: "https://api.github.com/repos/openshift/cincinnati-graph-data/commits/fef06adb57b9d965bfc9ae0959bd038f3044207e".to_string()
                },
                protected: true
            }];

        assert_eq!(branches_expected, branches);
    }
}
