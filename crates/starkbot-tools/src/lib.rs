pub mod api_key_read;
pub mod api_keys_check;
pub mod bash;
pub mod edit_file;
pub mod find_files;
pub mod grep;
pub mod install_integration;
pub mod list_files;
pub mod load_skill;
pub mod read_file;
pub mod sub_agent;
pub mod web_fetch;
pub mod write_file;
pub mod approval;
pub mod guard;
pub mod skill_test_tools;

use std::path::PathBuf;
use metalcraft::ToolRegistry;

/// Runtime configuration for tools that need it.
pub struct ToolConfig {
    pub api_key: String,
    pub model_name: String,
    pub system_prompt: String,
    pub skills_dir: PathBuf,
    pub available_skills: Vec<String>,
    pub keys_path: Option<PathBuf>,
    pub custom_dir: Option<PathBuf>,
    pub skill_tests_dir: Option<PathBuf>,
    pub data_root: Option<PathBuf>,
}

/// Register only the tools listed by name.
pub fn create_registry_for(tool_names: &[String]) -> ToolRegistry {
    create_registry_for_with_config(tool_names, None)
}

/// Register tools with optional config for tools that need runtime parameters.
pub fn create_registry_for_with_config(
    tool_names: &[String],
    config: Option<&ToolConfig>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    for name in tool_names {
        registry = match name.as_str() {
            "read_file" => registry.register(read_file::ReadFileTool),
            "write_file" => registry.register(write_file::WriteFileTool),
            "edit_file" => registry.register(edit_file::EditFileTool),
            "bash" => registry.register(bash::BashTool),
            "list_files" => registry.register(list_files::ListFilesTool),
            "grep" => registry.register(grep::GrepTool),
            "find_files" => registry.register(find_files::FindFilesTool),
            "load_skill" => {
                if let Some(cfg) = config {
                    registry.register(load_skill::LoadSkillTool::new(
                        cfg.skills_dir.clone(),
                        cfg.available_skills.clone(),
                    ))
                } else {
                    log::warn!("load_skill tool requires ToolConfig, skipping");
                    registry
                }
            }
            "web_fetch" => {
                if let Some(cfg) = config {
                    if let Some(ref keys_path) = cfg.keys_path {
                        registry.register(web_fetch::WebFetchTool::new(keys_path.clone()))
                    } else {
                        registry.register(web_fetch::WebFetchTool::default())
                    }
                } else {
                    registry.register(web_fetch::WebFetchTool::default())
                }
            }
            "api_keys_check" => {
                if let Some(cfg) = config {
                    if let Some(ref keys_path) = cfg.keys_path {
                        registry.register(api_keys_check::ApiKeysCheckTool::new(keys_path.clone()))
                    } else {
                        log::warn!("api_keys_check tool requires keys_path in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("api_keys_check tool requires ToolConfig, skipping");
                    registry
                }
            }
            "api_key_read" => {
                if let Some(cfg) = config {
                    if let Some(ref keys_path) = cfg.keys_path {
                        registry.register(api_key_read::ApiKeyReadTool::new(keys_path.clone()))
                    } else {
                        log::warn!("api_key_read tool requires keys_path in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("api_key_read tool requires ToolConfig, skipping");
                    registry
                }
            }
            "install_integration" => {
                if let Some(cfg) = config {
                    if let Some(ref data_root) = cfg.data_root {
                        registry.register(install_integration::InstallIntegrationTool::new(data_root.clone()))
                    } else {
                        log::warn!("install_integration tool requires data_root in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("install_integration tool requires ToolConfig, skipping");
                    registry
                }
            }
            "sub_agent" => {
                if let Some(cfg) = config {
                    registry.register(sub_agent::SubAgentTool::new(
                        cfg.api_key.clone(),
                        cfg.model_name.clone(),
                        cfg.system_prompt.clone(),
                    ))
                } else {
                    log::warn!("sub_agent tool requires ToolConfig, skipping");
                    registry
                }
            }
            "list_skill_tests" => {
                if let Some(cfg) = config {
                    if let Some(ref dir) = cfg.skill_tests_dir {
                        registry.register(skill_test_tools::ListSkillTestsTool::new(dir.clone()))
                    } else {
                        log::warn!("list_skill_tests tool requires skill_tests_dir in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("list_skill_tests tool requires ToolConfig, skipping");
                    registry
                }
            }
            "create_skill_test" => {
                if let Some(cfg) = config {
                    if let Some(ref dir) = cfg.skill_tests_dir {
                        registry.register(skill_test_tools::CreateSkillTestTool::new(dir.clone()))
                    } else {
                        log::warn!("create_skill_test tool requires skill_tests_dir in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("create_skill_test tool requires ToolConfig, skipping");
                    registry
                }
            }
            "edit_skill_test" => {
                if let Some(cfg) = config {
                    if let Some(ref dir) = cfg.skill_tests_dir {
                        registry.register(skill_test_tools::EditSkillTestTool::new(dir.clone()))
                    } else {
                        log::warn!("edit_skill_test tool requires skill_tests_dir in ToolConfig, skipping");
                        registry
                    }
                } else {
                    log::warn!("edit_skill_test tool requires ToolConfig, skipping");
                    registry
                }
            }
            unknown => {
                log::warn!("Unknown tool '{}' in persona, skipping", unknown);
                registry
            }
        };
    }
    registry
}

/// Extract (name, description) pairs from a tool registry.
/// Data-driven: descriptions come from each Tool's description() method.
pub fn tool_descriptions_from_registry(registry: &ToolRegistry) -> Vec<(String, String)> {
    let tools_json = registry.to_openai_tools();
    let mut descriptions: Vec<(String, String)> = tools_json.iter()
        .filter_map(|t| {
            let name = t["function"]["name"].as_str()?.to_string();
            let desc = t["function"]["description"].as_str()?.to_string();
            Some((name, desc))
        })
        .collect();
    descriptions.sort_by(|a, b| a.0.cmp(&b.0));
    descriptions
}

pub fn truncate_output(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    let half = max_chars / 2;
    let mut start_end = half;
    while !s.is_char_boundary(start_end) {
        start_end -= 1;
    }
    let mut tail_start = s.len() - half;
    while !s.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let omitted = s.len() - start_end - (s.len() - tail_start);
    format!(
        "{}\n\n... [truncated {} characters] ...\n\n{}",
        &s[..start_end],
        omitted,
        &s[tail_start..]
    )
}
