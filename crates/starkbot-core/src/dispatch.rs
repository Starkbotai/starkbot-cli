use metalcraft::{create_react_agent_with_hooks, AgentState, Executor, RunOutcome};
use rig::client::CompletionClient;
use rig::providers::openai;
use std::path::Path;
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
        let system_prompt = persona.build_system_prompt(skills_dir, cwd);
        let tool_config = starkbot_tools::ToolConfig {
            api_key: api_key.to_string(),
            model_name: model_name.to_string(),
            system_prompt: system_prompt.clone(),
            skills_dir: skills_dir.to_path_buf(),
            available_skills: persona.skills.clone(),
        };
        let registry = starkbot_tools::create_registry_for_with_config(&persona.tools, Some(&tool_config));
        let client = openai::Client::new(api_key)?;
        let model = client.completion_model(model_name);
        let hook = approval::build_hook(approval_mode);
        let graph = create_react_agent_with_hooks(model, registry, &system_prompt, hook)?.into_arc();
        let step_guard = guard::build_agent_guard(guard::GuardConfig::default());

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
