//! Brat Skills Library
//!
//! Embeds Claude Code skill definitions for each software factory role:
//! - bootstrap: Auto-scan and generate KB notes
//! - product: Product requirements analysis
//! - architecture: Architecture design
//! - implementation: Code implementation with TDD
//! - review: Code review and approval
//! - memory: Knowledge capture and promotion
//!
//! Skills are embedded into the binary at compile time via `rust-embed`
//! and installed to `~/.claude/skills/brat-*/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rust_embed::Embed;
use serde::{Deserialize, Serialize};

/// Errors raised by the skills crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no embedded skill found")]
    NoEmbeddedSkill,
    #[error("cannot determine Claude skills directory (no home dir)")]
    NoHomeDir,
    #[error("skill not installed")]
    NotInstalled,
    #[error("malformed SKILL.md frontmatter")]
    BadFrontmatter,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Embedded skill assets compiled into the binary.
#[derive(Embed)]
#[folder = "skills/"]
#[prefix = ""]
pub struct EmbeddedSkills;

/// Skill metadata parsed from SKILL.md YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// A complete skill with all its files.
#[derive(Debug, Clone)]
pub struct Skill {
    pub meta: SkillMeta,
    pub skill_md: String,
    pub files: HashMap<String, Vec<u8>>,
    pub checksum: String,
}

impl Skill {
    /// BLAKE3 checksum over the skill markdown plus companion files.
    pub fn compute_checksum(skill_md: &str, files: &HashMap<String, Vec<u8>>) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(skill_md.as_bytes());

        let mut paths: Vec<_> = files.keys().collect();
        paths.sort();
        for path in paths {
            hasher.update(path.as_bytes());
            if let Some(bytes) = files.get(path) {
                hasher.update(bytes);
            }
        }

        hasher.finalize().to_hex().to_string()
    }
}

/// Load an embedded skill by name.
pub fn load_skill(name: &str) -> Result<Skill> {
    let skill_path = format!("brat-{}/SKILL.md", name);
    let skill_md_content = EmbeddedSkills::get(&skill_path)
        .ok_or(Error::NoEmbeddedSkill)?;
    let skill_md = String::from_utf8_lossy(&skill_md_content.data).to_string();
    let meta = parse_skill_frontmatter(&skill_md)?;

    let mut files = HashMap::new();
    let prefix = format!("brat-{}/", name);
    for path in EmbeddedSkills::iter() {
        let p = path.as_ref();
        if p.starts_with(&prefix) && p != skill_path {
            if let Some(rel) = p.strip_prefix(&prefix) {
                if let Some(content) = EmbeddedSkills::get(p) {
                    files.insert(rel.to_string(), content.data.to_vec());
                }
            }
        }
    }

    let checksum = Skill::compute_checksum(&skill_md, &files);
    Ok(Skill {
        meta,
        skill_md,
        files,
        checksum,
    })
}

/// Parse YAML frontmatter from a SKILL.md.
pub fn parse_skill_frontmatter(content: &str) -> Result<SkillMeta> {
    let rest = content.strip_prefix("---").ok_or(Error::BadFrontmatter)?;
    let end = rest.find("---").ok_or(Error::BadFrontmatter)?;
    let yaml = rest[..end].trim();
    Ok(serde_yaml::from_str(yaml)?)
}

/// `~/.claude/skills` directory.
pub fn claude_skills_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".claude").join("skills"))
        .ok_or(Error::NoHomeDir)
}

/// Install a single skill.
pub fn install_skill(name: &str) -> Result<PathBuf> {
    let skill = load_skill(name)?;
    let target_dir = claude_skills_dir()?;
    let skill_dir = target_dir.join(&skill.meta.name);

    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(skill_dir.join("SKILL.md"), &skill.skill_md)?;
    for (rel, content) in &skill.files {
        let file_path = skill_dir.join(rel);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file_path, content)?;
    }

    Ok(skill_dir)
}

/// Install all brat skills.
pub fn install_all_skills() -> Result<Vec<(String, PathBuf)>> {
    let names = ["bootstrap", "product", "architecture", "implementation", "review", "memory"];
    let mut installed = Vec::new();
    for name in &names {
        match install_skill(name) {
            Ok(path) => installed.push((name.to_string(), path)),
            Err(e) => eprintln!("Warning: failed to install skill '{}': {}", name, e),
        }
    }
    Ok(installed)
}

/// Get info about an embedded skill.
#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub checksum: String,
    pub file_count: usize,
}

pub fn get_skill_info(name: &str) -> Result<SkillInfo> {
    let s = load_skill(name)?;
    Ok(SkillInfo {
        name: s.meta.name,
        version: s.meta.version,
        description: s.meta.description,
        tags: s.meta.tags,
        checksum: s.checksum,
        file_count: s.files.len() + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: test-skill\ndescription: A test skill\nversion: 1.0.0\ntags: [test, example]\n---\n\n# Test Skill\n";
        let meta = parse_skill_frontmatter(content).unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(meta.description, "A test skill");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_checksum_deterministic() {
        let mut a = HashMap::new();
        a.insert("a.md".to_string(), b"x".to_vec());
        a.insert("b.md".to_string(), b"y".to_vec());

        let mut b = HashMap::new();
        b.insert("b.md".to_string(), b"y".to_vec());
        b.insert("a.md".to_string(), b"x".to_vec());

        assert_eq!(
            Skill::compute_checksum("main", &a),
            Skill::compute_checksum("main", &b)
        );
    }
}
