use metalcraft::{create_react_agent_with_hooks, AgentState, Executor, RunOutcome};
use rig::client::CompletionClient;
use rig::providers::openai;
use starkbot_skills::SkillRegistry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::persona::Persona;
use crate::context::CompactionConfig;
use starkbot_tools::approval::{self, ApprovalMode};
use starkbot_tools::guard;

/// All state needed to run the agent.
pub struct AgentRunner {
    pub graph: Arc<metalcraft::CompiledGraph<AgentState>>,
    pub api_key: String,
    pub model_name: String,
    pub step_guard: metalcraft::StepGuard<AgentState>,
    pub compaction_config: CompactionConfig,
}

impl AgentRunner {
    /// Build an agent runner for a given persona.
    pub fn build(
        persona: &Persona,
        skills_dir: &Path,
        cwd: &str,
        api_key: &str,
        model_name: &str,
        approval_mode: ApprovalMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::build_with_keys(persona, skills_dir, cwd, api_key, model_name, approval_mode, None)
    }

    /// Build an agent runner with optional keys path and verbose guard output.
    pub fn build_with_keys(
        persona: &Persona,
        skills_dir: &Path,
        cwd: &str,
        api_key: &str,
        model_name: &str,
        approval_mode: ApprovalMode,
        keys_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::build_inner(persona, skills_dir, cwd, api_key, model_name, approval_mode, keys_path, true)
    }

    /// Build an agent runner for TUI mode (no stderr output from guard).
    pub fn build_for_tui(
        persona: &Persona,
        skills_dir: &Path,
        cwd: &str,
        api_key: &str,
        model_name: &str,
        approval_mode: ApprovalMode,
        keys_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::build_inner(persona, skills_dir, cwd, api_key, model_name, approval_mode, keys_path, false)
    }

    fn build_inner(
        persona: &Persona,
        skills_dir: &Path,
        cwd: &str,
        api_key: &str,
        model_name: &str,
        approval_mode: ApprovalMode,
        keys_path: Option<PathBuf>,
        verbose_guard: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load skill registry for tag-based discovery
        let skill_registry = SkillRegistry::load_from_dir(skills_dir);
        let skills = persona.resolved_skills(&skill_registry);
        // Skills enable their required tools (data-driven from skill frontmatter)
        let tools = persona.resolved_tools_with_skills(&skill_registry);

        // Build tool registry first (data-driven: descriptions live on each Tool impl)
        let custom_dir = keys_path.as_ref()
            .and_then(|p| p.parent())
            .map(|root| root.join("custom"));
        let tool_config = starkbot_tools::ToolConfig {
            api_key: api_key.to_string(),
            model_name: model_name.to_string(),
            system_prompt: String::new(), // placeholder — we'll set real prompt after
            skills_dir: skills_dir.to_path_buf(),
            available_skills: skills,
            keys_path,
            custom_dir,
        };
        let registry = starkbot_tools::create_registry_for_with_config(&tools, Some(&tool_config));

        // Extract tool descriptions from the actual registry (data-driven)
        let tool_descriptions = starkbot_tools::tool_descriptions_from_registry(&registry);

        // Build system prompt with real tool descriptions and discovered skills
        let system_prompt = persona.build_system_prompt_with_registry(
            skills_dir, cwd, &skill_registry, &tool_descriptions,
        );

        let client = openai::Client::new(api_key)?;
        let model = client.completion_model(model_name);
        let hook = approval::build_hook(approval_mode);
        let graph = create_react_agent_with_hooks(model, registry, &system_prompt, hook)?.into_arc();
        let step_guard = guard::build_agent_guard(guard::GuardConfig {
            verbose: verbose_guard,
            ..guard::GuardConfig::default()
        });

        Ok(Self {
            graph,
            api_key: api_key.to_string(),
            model_name: model_name.to_string(),
            step_guard,
            compaction_config: CompactionConfig::default(),
        })
    }

    /// Create a compaction model on demand.
    pub fn compaction_model(&self) -> Result<impl rig::completion::CompletionModel, Box<dyn std::error::Error>> {
        let client = openai::Client::new(&self.api_key)?;
        Ok(client.completion_model(&self.model_name))
    }

    /// Run a single turn and return the outcome.
    pub async fn run(&self, state: AgentState) -> Result<RunOutcome<AgentState>, metalcraft::GraphError> {
        let executor = Executor::new_from_arc(self.graph.clone())
            .max_steps(100)
            .with_step_guard(self.step_guard.clone());
        executor.run(state, "agent").await
    }
}
