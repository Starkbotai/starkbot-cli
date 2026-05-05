use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Persona {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    pub system_prompt: String,
}

impl Persona {
    pub fn load(slug: &str, personas_dir: &Path) -> Result<Self, String> {
        let file = personas_dir.join(format!("{}.json", slug));
        if !file.exists() {
            return Err(format!("Persona '{}' not found at {}", slug, file.display()));
        }
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;
        let persona: Persona = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", file.display(), e))?;
        Ok(persona)
    }

    pub fn list_available(personas_dir: &Path) -> Vec<String> {
        let entries = match std::fs::read_dir(personas_dir) {
            Ok(rd) => rd,
            Err(_) => return vec![],
        };
        let mut slugs: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) == Some("json") {
                    path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        slugs.sort();
        slugs
    }

    pub fn build_system_prompt(&self, skills_dir: &Path, cwd: &str) -> String {
        let mut prompt = self.system_prompt.clone();
        prompt.push_str(&format!("\n\nWorking directory: {}", cwd));

        if !self.skills.is_empty() {
            prompt.push_str("\n\n# Available Skills\n");
            prompt.push_str("You have access to the `load_skill` tool. Call it with a skill name to load detailed guidance.\n");
            prompt.push_str("Available skills:\n");
            for skill in &self.skills {
                let desc = load_skill_description(skill, skills_dir);
                prompt.push_str(&format!("- **{}**: {}\n", skill, desc));
            }
        }

        prompt
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
