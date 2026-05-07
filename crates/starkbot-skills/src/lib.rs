pub mod rag;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub requires_tools: Vec<String>,
    pub requires_keys: Vec<String>,
    pub content: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
}

impl SkillRegistry {
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut skills = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Some(skill) = Self::parse_skill_file(&path) {
                        skills.insert(skill.name.clone(), skill);
                    }
                }
            }
        }

        Self {
            skills,
            skills_dir: dir.to_path_buf(),
        }
    }

    /// Create a registry from a pre-built map (useful for testing).
    pub fn from_map(skills: HashMap<String, Skill>, skills_dir: PathBuf) -> Self {
        Self { skills, skills_dir }
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<_> = self.skills.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    pub fn reload(&mut self) {
        *self = Self::load_from_dir(&self.skills_dir);
    }

    fn parse_skill_file(path: &Path) -> Option<Skill> {
        let content = std::fs::read_to_string(path).ok()?;
        let name = path.file_stem()?.to_str()?.to_string();

        let (description, version, tags, requires_tools, requires_keys) = parse_frontmatter(&content);
        let body = strip_frontmatter(&content).to_string();

        Some(Skill {
            name,
            description,
            version,
            tags,
            requires_tools,
            requires_keys,
            content: body,
            file_path: path.to_path_buf(),
        })
    }
}

/// Parse YAML frontmatter for skill metadata.
fn parse_frontmatter(content: &str) -> (String, String, Vec<String>, Vec<String>, Vec<String>) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (String::new(), String::new(), vec![], vec![], vec![]);
    }

    let after_open = &trimmed[3..];
    let close_pos = match after_open.find("\n---") {
        Some(p) => p,
        None => return (String::new(), String::new(), vec![], vec![], vec![]),
    };
    let yaml_block = &after_open[..close_pos];

    let mut description = String::new();
    let mut version = String::new();
    let mut tags = vec![];
    let mut requires_tools = vec![];
    let mut requires_keys = vec![];

    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("description:") {
            description = rest.trim().trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("version:") {
            version = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("tags:") {
            tags = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("requires_tools:") {
            requires_tools = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("requires_keys:") {
            requires_keys = parse_yaml_list(rest);
        }
    }

    (description, version, tags, requires_tools, requires_keys)
}

fn parse_yaml_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.starts_with('[') && s.ends_with(']') {
        s[1..s.len() - 1]
            .split(',')
            .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![]
    }
}

/// Strip YAML frontmatter, returning just the body.
pub fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content;
    }
    let after_open = &trimmed[3..];
    match after_open.find("\n---") {
        Some(pos) => {
            let after_close = &after_open[pos + 4..];
            after_close.trim_start_matches('\n')
        }
        None => content,
    }
}

/// Extract `description` from YAML frontmatter.
pub fn parse_frontmatter_description(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_open = &trimmed[3..];
    let close_pos = after_open.find("\n---")?;
    let yaml_block = &after_open[..close_pos];
    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("description:") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\ndescription: test\n---\n# Body\nContent here";
        assert_eq!(strip_frontmatter(content), "# Body\nContent here");
    }

    #[test]
    fn test_parse_frontmatter_description() {
        let content = "---\ndescription: My skill\nversion: 1.0\n---\n# Body";
        assert_eq!(parse_frontmatter_description(content), Some("My skill".to_string()));
    }
}
