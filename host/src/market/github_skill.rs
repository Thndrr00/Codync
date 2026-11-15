//! A skill from a pasted public GitHub link: the folder that holds `SKILL.md`, its repository and branch.
//! Ported from `OpenBot` 0.69.0, "Add skills from GitHub".

use anyhow::{Result, bail};

/// Where a skill lives in a public repository.
#[derive(Debug, PartialEq, Eq)]
pub struct Link {
    /// `owner/repo`.
    pub repo: String,
    /// The branch the link names.
    pub branch: String,
    /// The folder that holds `SKILL.md`, empty when the repository's top is the skill.
    pub path: String,
}

impl Link {
    /// What the installed skill records as its `source`: the folder, in a form that can be read back.
    pub fn source(&self) -> String {
        if self.path.is_empty() {
            format!("https://github.com/{}/tree/{}", self.repo, self.branch)
        } else {
            format!("https://github.com/{}/tree/{}/{}", self.repo, self.branch, self.path)
        }
    }
}

/// Reads the link a person copied from GitHub: a repository, a folder (`/tree/…`) or `SKILL.md`
/// (`/blob/…`), whose folder is then the skill. A file that is not `SKILL.md` is refused, because a skill
/// is a folder. A branch with a `/` in it is read up to its first segment, as GitHub's own page does.
pub fn parse(url: &str) -> Result<Link> {
    let rest = url.trim().trim_end_matches('/');
    let rest = rest.strip_prefix("https://").or_else(|| rest.strip_prefix("http://")).unwrap_or(rest);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let Some(rest) = rest.strip_prefix("github.com/") else {
        bail!("Use a public GitHub link, such as https://github.com/owner/repo/tree/main/skills/name");
    };
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/').filter(|part| !part.is_empty());
    let Some(owner) = parts.next() else { bail!("The link names no repository") };
    let Some(repo) = parts.next() else { bail!("The link names no repository") };
    let repo = format!("{owner}/{repo}");
    let Some(kind) = parts.next() else {
        // The repository's own page: its top folder is the skill, which the download finds out.
        return Ok(Link { repo, branch: "main".into(), path: String::new() });
    };
    if kind != "tree" && kind != "blob" {
        bail!("Open the repository, or the folder that holds the skill, and copy that link");
    }
    let Some(branch) = parts.next() else { bail!("The link names no branch") };
    let path: Vec<&str> = parts.collect();
    let mut path = path.join("/");
    if kind == "blob" {
        let file = path.rsplit('/').next().unwrap_or_default().to_owned();
        if file != "SKILL.md" {
            bail!("Pick the folder that holds SKILL.md, or SKILL.md itself, not another file");
        }
        path = path.rsplit_once('/').map(|(folder, _)| folder.to_owned()).unwrap_or_default();
    }
    Ok(Link { repo, branch: branch.to_owned(), path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    fn parsed(url: &str) -> Result<Link, anyhow::Error> {
        parse(url).with_context(|| url.to_owned())
    }

    #[test]
    fn a_folder_link_names_its_repository_branch_and_path() {
        let link = parsed("https://github.com/anthropics/skills/tree/main/skills/pdf").unwrap();
        assert_eq!(link.repo, "anthropics/skills");
        assert_eq!(link.branch, "main");
        assert_eq!(link.path, "skills/pdf");
        assert_eq!(link.source(), "https://github.com/anthropics/skills/tree/main/skills/pdf");
    }

    #[test]
    fn a_skill_file_link_names_its_folder() {
        let link = parsed("https://github.com/owner/repo/tree/stable/skills/reports").unwrap();
        assert_eq!(link.branch, "stable");
        assert_eq!(link.path, "skills/reports");
        let pointed = parsed("https://www.github.com/owner/repo/blob/stable/skills/reports/SKILL.md/").unwrap();
        assert_eq!(pointed, link, "the file and its folder are the same skill");
        let root = parsed("https://github.com/owner/repo/blob/main/SKILL.md").unwrap();
        assert_eq!(root.path, "", "a SKILL.md at the top makes the repository the skill");
    }

    #[test]
    fn a_repository_link_is_installed_from_its_top() {
        let link = parsed("https://github.com/owner/repo.git").unwrap();
        assert_eq!((link.repo.as_str(), link.branch.as_str(), link.path.as_str()), ("owner/repo", "main", ""));
    }

    #[test]
    fn anything_that_is_not_a_public_github_folder_is_refused() {
        for bad in [
            "https://gitlab.com/owner/repo/tree/main/skill",
            "owner/repo",
            "https://github.com/owner",
            "https://github.com/owner/repo/issues/1",
            "https://github.com/owner/repo/blob/main/README.md",
        ] {
            assert!(parse(bad).is_err(), "{bad} must not install");
        }
        // The repository's root on a branch is a folder like any other.
        assert_eq!(parsed("https://github.com/owner/repo/tree/main").unwrap().path, "");
    }
}
