use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Tool group names map to predefined sets of tools.
pub fn resolve_tool_group(group: &str) -> Vec<String> {
    match group {
        "filesystem" => vec![
            "read_file", "write_file", "edit_file", "list_files",
        ],
        "search" => vec![
            "grep", "find_files",
        ],
        "execution" => vec![
            "bash",
        ],
        "web" => vec![
            "web_fetch",
        ],
        "skills" => vec![
            "load_skill",
        ],
        "delegation" => vec![
            "sub_agent",
        ],
        "api_keys" => vec![
            "api_keys_check", "install_api_key",
        ],
        _ => {
            log::warn!("Unknown tool group: {}", group);
            vec![]
        }
    }.into_iter().map(String::from).collect()
}

#[derive(Debug, Clone)]
pub struct Persona {
    pub key: String,
    pub label: String,
    pub description: String,
    pub emoji: String,
    pub version: String,
    pub tool_groups: Vec<String>,
    pub additional_tools: Vec<String>,
    pub skill_tags: Vec<String>,
    pub explicit_skills: Vec<String>,
    pub aliases: Vec<String>,
    pub max_iterations: usize,
    pub sort_order: i32,
    pub enabled: bool,
    pub system_prompt: String,

    // Legacy compat fields (from JSON personas)
    legacy_tools: Vec<String>,
    legacy_skills: Vec<String>,
}

/// Temporary struct for deserializing legacy JSON personas.
#[derive(Debug, Deserialize)]
struct LegacyPersona {
    name: String,
    description: String,
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    system_prompt: String,
}

impl Persona {
    /// Base tools from tool_groups + additional_tools + legacy_tools.
    fn base_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = Vec::new();

        for group in &self.tool_groups {
            for tool in resolve_tool_group(group) {
                if !tools.contains(&tool) {
                    tools.push(tool);
                }
            }
        }

        for tool in &self.additional_tools {
            if !tools.contains(tool) {
                tools.push(tool.clone());
            }
        }

        for tool in &self.legacy_tools {
            if !tools.contains(tool) {
                tools.push(tool.clone());
            }
        }

        tools
    }

    /// All resolved tool names: base tools + tools required by resolved skills.
    /// Skills enable their required tools automatically (data-driven from skill frontmatter).
    pub fn resolved_tools_with_skills(&self, registry: &starkbot_skills::SkillRegistry) -> Vec<String> {
        let mut tools = self.base_tools();

        // Skills enable their required tools
        let skills = self.resolved_skills(registry);
        for skill_name in &skills {
            if let Some(skill) = registry.get(skill_name) {
                for tool in &skill.requires_tools {
                    if !tools.contains(tool) {
                        tools.push(tool.clone());
                    }
                }
            }
        }

        tools
    }

    /// All resolved tool names (without skill registry — uses base tools only).
    pub fn resolved_tools(&self) -> Vec<String> {
        self.base_tools()
    }

    /// All resolved skill names for this persona.
    /// Uses explicit_skills plus any skills discovered via skill_tags from the registry.
    pub fn resolved_skills(&self, registry: &starkbot_skills::SkillRegistry) -> Vec<String> {
        let mut skills: Vec<String> = self.explicit_skills.clone();

        // Add legacy skills
        for s in &self.legacy_skills {
            if !skills.contains(s) {
                skills.push(s.clone());
            }
        }

        // Add skills matching any of our skill_tags
        if !self.skill_tags.is_empty() {
            for skill in registry.all() {
                if skills.contains(&skill.name) {
                    continue;
                }
                for tag in &skill.tags {
                    if self.skill_tags.contains(tag) {
                        skills.push(skill.name.clone());
                        break;
                    }
                }
            }
        }

        skills.sort();
        skills.dedup();
        skills
    }

    /// Convenience: get tools and skills as the old format expected.
    pub fn tools(&self) -> Vec<String> {
        self.resolved_tools()
    }

    pub fn skills_list(&self) -> Vec<String> {
        self.explicit_skills.clone()
    }

    /// Load a persona. Tries agents/{slug}/agent.md first, then personas/{slug}.json.
    pub fn load(slug: &str, personas_dir: &Path) -> Result<Self, String> {
        // Try markdown agent format first
        let agents_dir = personas_dir.parent().unwrap_or(Path::new(".")).join("agents");
        let agent_md = agents_dir.join(slug).join("agent.md");
        if agent_md.exists() {
            return Self::load_from_markdown(slug, &agent_md);
        }

        // Fall back to legacy JSON
        let json_file = personas_dir.join(format!("{}.json", slug));
        if json_file.exists() {
            return Self::load_from_json(slug, &json_file);
        }

        Err(format!("Persona '{}' not found (checked {} and {})", slug, agent_md.display(), json_file.display()))
    }

    fn load_from_json(slug: &str, path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let legacy: LegacyPersona = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        Ok(Self {
            key: slug.to_string(),
            label: legacy.name.clone(),
            description: legacy.description,
            emoji: String::new(),
            version: "1.0.0".to_string(),
            tool_groups: vec![],
            additional_tools: vec![],
            skill_tags: vec![],
            explicit_skills: vec![],
            aliases: vec![],
            max_iterations: 100,
            sort_order: 50,
            enabled: true,
            system_prompt: legacy.system_prompt,
            legacy_tools: legacy.tools,
            legacy_skills: legacy.skills,
        })
    }

    fn load_from_markdown(slug: &str, path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let (frontmatter, body) = parse_agent_frontmatter(&content);

        Ok(Self {
            key: slug.to_string(),
            label: frontmatter.label.unwrap_or_else(|| slug.to_string()),
            description: frontmatter.description.unwrap_or_default(),
            emoji: frontmatter.emoji.unwrap_or_default(),
            version: frontmatter.version.unwrap_or_else(|| "1.0.0".to_string()),
            tool_groups: frontmatter.tool_groups,
            additional_tools: frontmatter.additional_tools,
            skill_tags: frontmatter.skill_tags,
            explicit_skills: frontmatter.explicit_skills,
            aliases: frontmatter.aliases,
            max_iterations: frontmatter.max_iterations.unwrap_or(100),
            sort_order: frontmatter.sort_order.unwrap_or(50),
            enabled: frontmatter.enabled.unwrap_or(true),
            system_prompt: body,
            legacy_tools: vec![],
            legacy_skills: vec![],
        })
    }

    pub fn list_available(personas_dir: &Path) -> Vec<String> {
        let mut slugs: Vec<String> = Vec::new();

        // Check agents/ directory
        let agents_dir = personas_dir.parent().unwrap_or(Path::new(".")).join("agents");
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() && path.join("agent.md").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        slugs.push(name.to_string());
                    }
                }
            }
        }

        // Check personas/ directory (legacy JSON)
        if let Ok(entries) = std::fs::read_dir(personas_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|x| x.to_str()) == Some("json") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if !slugs.contains(&name.to_string()) {
                            slugs.push(name.to_string());
                        }
                    }
                }
            }
        }

        slugs.sort();
        slugs
    }

    /// Build system prompt with tag-based skill discovery and data-driven tool descriptions.
    ///
    /// `tool_descriptions` comes from the actual metalcraft ToolRegistry — each tool's
    /// description is defined on its Tool impl, not hardcoded here.
    pub fn build_system_prompt_with_registry(
        &self,
        skills_dir: &Path,
        cwd: &str,
        skill_registry: &starkbot_skills::SkillRegistry,
        tool_descriptions: &[(String, String)],
    ) -> String {
        let skills = self.resolved_skills(skill_registry);
        self.build_system_prompt_inner(skills_dir, cwd, &skills, tool_descriptions)
    }

    /// Build system prompt without a registry (legacy/fallback).
    pub fn build_system_prompt(&self, skills_dir: &Path, cwd: &str) -> String {
        let skills = if !self.legacy_skills.is_empty() {
            self.legacy_skills.clone()
        } else {
            self.explicit_skills.clone()
        };
        self.build_system_prompt_inner(skills_dir, cwd, &skills, &[])
    }

    fn build_system_prompt_inner(
        &self,
        skills_dir: &Path,
        cwd: &str,
        skills: &[String],
        tool_descriptions: &[(String, String)],
    ) -> String {
        let mut prompt = self.system_prompt.clone();
        prompt.push_str(&format!("\n\nWorking directory: {}", cwd));

        // Build tools text from data-driven descriptions (from metalcraft Tool::description())
        let tools_text = if tool_descriptions.is_empty() {
            String::new()
        } else {
            let mut t = String::from("IMPORTANT: Always use your tools to take action. Do not give generic advice when you have a tool that can do the job. Act, don't advise.\n\n");
            for (name, desc) in tool_descriptions {
                t.push_str(&format!("- **{}**: {}\n", name, desc));
            }
            t
        };

        // Replace {tools} placeholder if present, otherwise append
        if prompt.contains("{tools}") {
            prompt = prompt.replace("{tools}", &tools_text);
        } else if !tools_text.is_empty() {
            prompt.push_str("\n\n# Available Tools\n");
            prompt.push_str(&tools_text);
        }

        // Skills section — data-driven from skill file frontmatter
        if !skills.is_empty() {
            prompt.push_str("\n\n# Available Skills\n");
            prompt.push_str("Use the `load_skill` tool to load detailed guidance for any of these skills.\n");
            prompt.push_str("Available skills:\n");
            for skill in skills {
                let desc = load_skill_description(skill, skills_dir);
                prompt.push_str(&format!("- **{}**: {}\n", skill, desc));
            }
        }

        // Replace {available_skills} placeholder if present
        if prompt.contains("{available_skills}") {
            let skills_text = if skills.is_empty() {
                "No skills currently loaded.".to_string()
            } else {
                skills.iter()
                    .map(|s| {
                        let desc = load_skill_description(s, skills_dir);
                        format!("- **{}**: {}", s, desc)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            prompt = prompt.replace("{available_skills}", &skills_text);
        }

        prompt
    }

    // Compat accessors for code that reads .name
    pub fn name(&self) -> &str {
        &self.label
    }

    pub fn default_personas_dir() -> PathBuf {
        let cwd_based = PathBuf::from("personas");
        if cwd_based.is_dir() { return cwd_based; }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let exe_based = parent.join("personas");
                if exe_based.is_dir() { return exe_based; }
            }
        }
        cwd_based
    }

    pub fn default_skills_dir() -> PathBuf {
        let cwd_based = PathBuf::from("skills");
        if cwd_based.is_dir() { return cwd_based; }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let exe_based = parent.join("skills");
                if exe_based.is_dir() { return exe_based; }
            }
        }
        cwd_based
    }

    pub fn default_agents_dir() -> PathBuf {
        let cwd_based = PathBuf::from("agents");
        if cwd_based.is_dir() { return cwd_based; }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let exe_based = parent.join("agents");
                if exe_based.is_dir() { return exe_based; }
            }
        }
        cwd_based
    }
}

fn load_skill_description(name: &str, skills_dir: &Path) -> String {
    let file = skills_dir.join(format!("{}.md", name));
    let content = match std::fs::read_to_string(&file) {
        Ok(c) => c,
        Err(_) => return "Specialized guidance".to_string(),
    };
    starkbot_skills::parse_frontmatter_description(&content)
        .unwrap_or_else(|| "Specialized guidance".to_string())
}

// --- Agent markdown frontmatter parsing ---

#[derive(Debug, Default)]
struct AgentFrontmatter {
    label: Option<String>,
    description: Option<String>,
    emoji: Option<String>,
    version: Option<String>,
    tool_groups: Vec<String>,
    additional_tools: Vec<String>,
    skill_tags: Vec<String>,
    explicit_skills: Vec<String>,
    aliases: Vec<String>,
    max_iterations: Option<usize>,
    sort_order: Option<i32>,
    enabled: Option<bool>,
}

fn parse_agent_frontmatter(content: &str) -> (AgentFrontmatter, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (AgentFrontmatter::default(), content.to_string());
    }

    let after_open = &trimmed[3..];
    let close_pos = match after_open.find("\n---") {
        Some(p) => p,
        None => return (AgentFrontmatter::default(), content.to_string()),
    };
    let yaml_block = &after_open[..close_pos];
    let body = after_open[close_pos + 4..].trim_start_matches('\n').to_string();

    let mut fm = AgentFrontmatter::default();

    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("label:") {
            fm.label = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("description:") {
            fm.description = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("emoji:") {
            fm.emoji = Some(rest.trim().trim_matches('"').to_string());
        } else if let Some(rest) = line.strip_prefix("version:") {
            fm.version = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("tool_groups:") {
            fm.tool_groups = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("additional_tools:") {
            fm.additional_tools = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("skill_tags:") {
            fm.skill_tags = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("explicit_skills:") {
            fm.explicit_skills = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("aliases:") {
            fm.aliases = parse_yaml_list(rest);
        } else if let Some(rest) = line.strip_prefix("max_iterations:") {
            fm.max_iterations = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("sort_order:") {
            fm.sort_order = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("enabled:") {
            fm.enabled = Some(rest.trim() == "true");
        }
    }

    (fm, body)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_tool_groups() {
        let fs_tools = resolve_tool_group("filesystem");
        assert!(fs_tools.contains(&"read_file".to_string()));
        assert!(fs_tools.contains(&"write_file".to_string()));

        let api_tools = resolve_tool_group("api_keys");
        assert!(api_tools.contains(&"api_keys_check".to_string()));
        assert!(api_tools.contains(&"install_api_key".to_string()));

        assert!(resolve_tool_group("unknown").is_empty());
    }

    #[test]
    fn test_parse_agent_frontmatter() {
        let content = r#"---
label: "DevOps Agent"
description: "Infrastructure specialist"
emoji: "🔧"
tool_groups: [filesystem, search, execution, web, api_keys]
skill_tags: [infrastructure, devops]
explicit_skills: [planning]
sort_order: 20
enabled: true
---
You are a DevOps specialist agent.
"#;
        let (fm, body) = parse_agent_frontmatter(content);
        assert_eq!(fm.label.unwrap(), "DevOps Agent");
        assert_eq!(fm.description.unwrap(), "Infrastructure specialist");
        assert_eq!(fm.emoji.unwrap(), "🔧");
        assert_eq!(fm.tool_groups, vec!["filesystem", "search", "execution", "web", "api_keys"]);
        assert_eq!(fm.skill_tags, vec!["infrastructure", "devops"]);
        assert_eq!(fm.explicit_skills, vec!["planning"]);
        assert_eq!(fm.sort_order, Some(20));
        assert_eq!(fm.enabled, Some(true));
        assert!(body.starts_with("You are a DevOps"));
    }

    #[test]
    fn test_resolved_tools_dedup() {
        let persona = Persona {
            key: "test".into(),
            label: "Test".into(),
            description: String::new(),
            emoji: String::new(),
            version: "1.0.0".into(),
            tool_groups: vec!["filesystem".into(), "search".into()],
            additional_tools: vec!["bash".into(), "read_file".into()], // read_file is in filesystem group
            skill_tags: vec![],
            explicit_skills: vec![],
            aliases: vec![],
            max_iterations: 100,
            sort_order: 50,
            enabled: true,
            system_prompt: String::new(),
            legacy_tools: vec![],
            legacy_skills: vec![],
        };
        let tools = persona.resolved_tools();
        let read_count = tools.iter().filter(|t| t.as_str() == "read_file").count();
        assert_eq!(read_count, 1, "read_file should appear exactly once");
        assert!(tools.contains(&"bash".to_string()));
    }
}
