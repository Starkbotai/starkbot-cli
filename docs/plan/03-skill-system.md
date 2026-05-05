# 03 - Skill System

## Overview

Skills are markdown-based declarative workflow documents that give the agent specialized knowledge and methodology for specific tasks. They are loaded on-demand and injected into the agent's context.

## Skill Format

Identical to StarkBot's format — markdown with YAML frontmatter:

```markdown
---
name: debugging
description: "Systematic approach to debugging code issues"
version: 1.2.0
requires_tools: [read_file, grep, bash, edit_file]
tags: [workflow, methodology, debugging]
metadata:
  category: development
  complexity: intermediate
---

# Debugging Methodology

## Step 1: Reproduce
- Identify the exact error message or unexpected behavior
- Create a minimal reproduction case

## Step 2: Investigate
- Read relevant source files
- Search for error patterns with grep
- Check recent changes (git log, git diff)

## Step 3: Hypothesize
- Form a theory about the root cause
- Identify the specific code path

## Step 4: Fix & Verify
- Apply the minimal fix
- Run tests to confirm the fix
- Check for regressions
```

## Skill Registry

```rust
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
    embeddings: Option<EmbeddingIndex>,  // For semantic skill search
}

pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub requires_tools: Vec<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub content: String,  // Full markdown body
    pub file_path: PathBuf,
}

impl SkillRegistry {
    /// Load all skills from directory
    pub fn load_from_dir(dir: &Path) -> Result<Self>;

    /// Hot-reload changed skills
    pub fn reload(&mut self) -> Result<Vec<String>>;

    /// Get skill by name
    pub fn get(&self, name: &str) -> Option<&Skill>;

    /// Search skills by query (semantic + tag-based)
    pub fn search(&self, query: &str) -> Vec<&Skill>;

    /// List skills matching tags
    pub fn by_tags(&self, tags: &[&str]) -> Vec<&Skill>;

    /// Get skills available for a persona
    pub fn for_persona(&self, persona: &Persona) -> Vec<&Skill>;
}
```

## Loading Mechanism

Skills are **not** preloaded into context. Instead, the agent has a `load_skill` tool:

```rust
pub struct LoadSkillTool {
    registry: Arc<SkillRegistry>,
}

impl Tool for LoadSkillTool {
    fn name(&self) -> &str { "load_skill" }

    fn description(&self) -> &str {
        "Load a methodology guide/skill document for specialized tasks. \
         Available skills are listed in your system prompt."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill to load"
                }
            },
            "required": ["skill_name"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let name = args["skill_name"].as_str().unwrap();
        match self.registry.get(name) {
            Some(skill) => Ok(json!({ "content": skill.content })),
            None => Ok(json!({ "error": format!("Skill '{}' not found", name) })),
        }
    }
}
```

## System Prompt Integration

The system prompt includes available skills as a menu:

```
## Available Skills

You can load detailed methodology guides using the `load_skill` tool:

- **planning** — Structured approach to task planning and decomposition
- **debugging** — Systematic debugging methodology
- **code-review** — Code quality review checklist
- **research** — Research and information gathering methodology
- **commit-message** — Git commit message best practices

Load a skill when you need specialized guidance for a task.
```

## Skill Graph (Relationships)

Skills have relationships that are visualized in the graf-rs graph viewer:

```rust
pub struct SkillGraph {
    nodes: Vec<SkillNode>,
    edges: Vec<SkillEdge>,
}

pub struct SkillNode {
    pub skill_name: String,
    pub category: String,
    pub enabled: bool,
}

pub enum SkillEdgeKind {
    DependsOn,      // Skill A requires Skill B
    RelatedTo,      // Conceptual relationship
    Precedes,       // Workflow ordering
    Extends,        // Skill A extends Skill B
}

pub struct SkillEdge {
    pub from: String,
    pub to: String,
    pub kind: SkillEdgeKind,
}
```

## Skill Categories (Planned)

### Development
- `planning` — Task decomposition and planning
- `debugging` — Systematic debugging
- `code-review` — Quality review
- `testing` — Test writing methodology
- `refactoring` — Safe refactoring patterns
- `commit-message` — Commit conventions

### Operations
- `ci-cd` — CI/CD pipeline management
- `dockerfile` — Container best practices
- `deployment` — Deployment workflows
- `monitoring` — Observability setup

### Research
- `research-methodology` — Information gathering
- `explore-codebase` — Codebase exploration
- `summarize` — Document summarization

### Creative
- `video-scripting` — Video content creation
- `documentation` — Technical writing

### Web3 (Optional Plugin)
- `swap` — DEX token swap workflow
- `aave` — Lending protocol interactions
- `safe-wallet` — Multi-sig management

## Hot-Reload

Skills can be modified on disk and reloaded without restarting:

```rust
// File watcher (notify crate)
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(&skills_dir, RecursiveMode::Recursive)?;

// On change event
for event in rx {
    if event.paths.iter().any(|p| p.extension() == Some("md".as_ref())) {
        registry.reload()?;
        app_tx.send(AppEvent::SkillsReloaded)?;
    }
}
```
