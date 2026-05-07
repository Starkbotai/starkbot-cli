pub mod views;
pub mod widgets;
pub mod theme;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};

use starkbot_api::events::{BackendEvent, FrontendCommand};
use starkbot_api::types::{AppSnapshot, SessionSummary, ChatSession, FlowSummary};
use starkbot_core::persona::Persona;
use starkbot_graph::{GraphData, GraphWidget, Viewport};
use starkbot_skills::Skill;

use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveView {
    Chat,
    Skills,
    Graph,
    Personas,
    Memory,
    Data,
    Scheduling,
    ApiKeys,
    Packs,
    Settings,
}

impl ActiveView {
    pub fn titles() -> Vec<&'static str> {
        vec!["Chat", "Skills", "Graph", "Personas", "Memory", "Data", "Scheduling", "API Keys", "Packs", "Settings"]
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Chat => 0,
            Self::Skills => 1,
            Self::Graph => 2,
            Self::Personas => 3,
            Self::Memory => 4,
            Self::Data => 5,
            Self::Scheduling => 6,
            Self::ApiKeys => 7,
            Self::Packs => 8,
            Self::Settings => 9,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Skills,
            2 => Self::Graph,
            3 => Self::Personas,
            4 => Self::Memory,
            5 => Self::Data,
            6 => Self::Scheduling,
            7 => Self::ApiKeys,
            8 => Self::Packs,
            9 => Self::Settings,
            _ => Self::Chat,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index((self.index() + 1) % 10)
    }
}

/// A chat message in the TUI.
#[derive(Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Input mode for API keys add flow.
#[derive(Clone, Copy, PartialEq)]
enum ApiKeyInputMode {
    Normal,
    EnteringName,
    EnteringKey,
}

/// A pending tool approval being shown in the TUI.
pub struct PendingApproval {
    pub request_id: String,
    pub tool_name: String,
    pub args_display: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsSection {
    AgentModel,
    ExtensionServer,
}

impl SettingsSection {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AgentModel => "Agent Model",
            Self::ExtensionServer => "Extensions",
        }
    }

    pub fn all() -> &'static [SettingsSection] {
        &[SettingsSection::AgentModel, SettingsSection::ExtensionServer]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsFocus {
    Sidebar,
    Content,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DataFocus {
    Sidebar,
    SessionList,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FlowsFocus {
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PacksFocus {
    List,
    Detail,
}

/// TUI application state.
pub struct TuiState {
    pub active_view: ActiveView,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub input_cursor: usize,
    pub persona_name: String,
    pub model_name: String,
    pub status: String,
    pub tool_activity: Vec<String>,
    pub skill_graph: GraphData,
    pub graph_viewport: Viewport,
    pub skill_names: Vec<String>,
    pub skills: Vec<Skill>,
    pub selected_skill: usize,
    pub personas: Vec<Persona>,
    pub selected_persona: usize,
    pub should_quit: bool,
    pub agent_busy: bool,
    // API keys state
    pub api_keys: Vec<(String, String, String)>, // (name, masked_key, updated_at)
    pub selected_api_key: usize,
    // API key add flow
    api_key_input_mode: ApiKeyInputMode,
    api_key_name_input: String,
    api_key_value_input: String,
    // Settings state
    pub available_models: Vec<String>,
    pub selected_model: usize,
    // Approval prompt state
    pub pending_approval: Option<PendingApproval>,
    // Chat scroll: offset from bottom (0 = pinned to bottom, higher = scrolled up)
    pub chat_scroll_up: u16,
    // Directory paths for open-folder
    pub skills_dir: PathBuf,
    pub agents_dir: PathBuf,
    // Settings sidebar
    pub settings_section: SettingsSection,
    pub settings_focus: SettingsFocus,
    // Data (sessions) state
    pub sessions: Vec<SessionSummary>,
    pub selected_session: usize,
    pub viewing_session: Option<ChatSession>,
    pub data_focus: DataFocus,
    // Flows state
    pub flows: Vec<FlowSummary>,
    pub selected_flow: usize,
    pub flows_focus: FlowsFocus,
    // Packs state
    pub remote_packs: Vec<starkbot_api::types::PackInfo>,
    pub selected_pack: usize,
    pub packs_focus: PacksFocus,
    pub packs_search: String,
    pub packs_loading: bool,
    pub packs_message: Option<String>,
    pub extension_server: String,
}

impl TuiState {
    pub fn new(persona_name: &str, model_name: &str) -> Self {
        let available_models = vec![
            "gpt-5.4".to_string(),
            "gpt-5.4-mini".to_string(),
            "gpt-5.5".to_string(),
        ];
        let selected_model = available_models.iter()
            .position(|m| m == model_name)
            .unwrap_or(0);
        Self {
            active_view: ActiveView::Chat,
            messages: vec![],
            input: String::new(),
            input_cursor: 0,
            persona_name: persona_name.to_string(),
            model_name: model_name.to_string(),
            status: "Ready".to_string(),
            tool_activity: vec![],
            skill_graph: GraphData::default(),
            graph_viewport: Viewport::default(),
            skill_names: vec![],
            skills: vec![],
            selected_skill: 0,
            personas: vec![],
            selected_persona: 0,
            should_quit: false,
            agent_busy: false,
            api_keys: vec![],
            selected_api_key: 0,
            api_key_input_mode: ApiKeyInputMode::Normal,
            api_key_name_input: String::new(),
            api_key_value_input: String::new(),
            available_models,
            selected_model,
            pending_approval: None,
            chat_scroll_up: 0,
            skills_dir: PathBuf::new(),
            agents_dir: PathBuf::new(),
            settings_section: SettingsSection::AgentModel,
            settings_focus: SettingsFocus::Content,
            sessions: vec![],
            selected_session: 0,
            viewing_session: None,
            data_focus: DataFocus::SessionList,
            flows: vec![],
            selected_flow: 0,
            flows_focus: FlowsFocus::List,
            remote_packs: vec![],
            selected_pack: 0,
            packs_focus: PacksFocus::List,
            packs_search: String::new(),
            packs_loading: false,
            packs_message: None,
            extension_server: "https://hyperpacks.org".to_string(),
        }
    }

    /// Create TuiState from an AppSnapshot (initial state from backend).
    pub fn from_snapshot(snapshot: &AppSnapshot) -> Self {
        let selected_model = snapshot.available_models.iter()
            .position(|m| *m == snapshot.model_name)
            .unwrap_or(0);

        let skills: Vec<Skill> = snapshot.skills.iter().map(|s| Skill {
            name: s.name.clone(),
            description: s.description.clone(),
            version: s.version.clone(),
            tags: s.tags.clone(),
            requires_tools: s.requires_tools.clone(),
            requires_keys: s.requires_keys.clone(),
            content: s.content.clone(),
            file_path: PathBuf::new(),
        }).collect();
        let skill_names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();

        // Convert persona infos to Persona objects for rendering
        let personas: Vec<Persona> = snapshot.personas.iter().map(|p| {
            Persona::from_display_info(
                &p.key, &p.label, &p.description, &p.emoji,
                p.enabled, &p.tool_groups, &p.skill_tags,
                &p.system_prompt_preview,
            )
        }).collect();

        // Build graph data
        let mut graph = GraphData::default();
        for n in &snapshot.graph_nodes {
            graph.nodes.push(starkbot_graph::GraphNode {
                id: n.id.clone(),
                label: n.label.clone(),
                category: n.category.clone(),
                weight: n.weight,
            });
        }
        for e in &snapshot.graph_edges {
            graph.edges.push(starkbot_graph::GraphEdge {
                from: e.from.clone(),
                to: e.to.clone(),
                label: e.label.clone(),
                kind: e.kind.clone(),
                weight: e.weight,
            });
        }

        let api_keys: Vec<(String, String, String)> = snapshot.api_keys.iter()
            .map(|k| (k.name.clone(), k.masked_key.clone(), String::new()))
            .collect();

        let messages: Vec<ChatMessage> = snapshot.messages.iter()
            .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
            .collect();

        Self {
            active_view: ActiveView::Chat,
            messages,
            input: String::new(),
            input_cursor: 0,
            persona_name: snapshot.persona_name.clone(),
            model_name: snapshot.model_name.clone(),
            status: snapshot.status.clone(),
            tool_activity: snapshot.tool_activity.clone(),
            skill_graph: graph,
            graph_viewport: Viewport::default(),
            skill_names,
            skills,
            selected_skill: 0,
            personas,
            selected_persona: 0,
            should_quit: false,
            agent_busy: snapshot.agent_busy,
            api_keys,
            selected_api_key: 0,
            api_key_input_mode: ApiKeyInputMode::Normal,
            api_key_name_input: String::new(),
            api_key_value_input: String::new(),
            available_models: snapshot.available_models.clone(),
            selected_model,
            pending_approval: None,
            chat_scroll_up: 0,
            skills_dir: PathBuf::from(&snapshot.skills_dir),
            agents_dir: PathBuf::from(&snapshot.agents_dir),
            settings_section: SettingsSection::AgentModel,
            settings_focus: SettingsFocus::Content,
            sessions: snapshot.sessions.clone(),
            selected_session: 0,
            viewing_session: None,
            data_focus: DataFocus::SessionList,
            flows: vec![],
            selected_flow: 0,
            flows_focus: FlowsFocus::List,
            remote_packs: vec![],
            selected_pack: 0,
            packs_focus: PacksFocus::List,
            packs_search: String::new(),
            packs_loading: false,
            packs_message: None,
            extension_server: snapshot.extension_server.clone(),
        }
    }

    /// Apply a BackendEvent to update TUI state.
    pub fn apply_event(&mut self, event: &BackendEvent) {
        match event {
            BackendEvent::ToolCall { name, args } => {
                self.add_tool_activity(&format!("▶ {} {}", name, args));
                self.add_message("tool", &format!("▶ {}({})", name, truncate_str(args, 80)));
            }
            BackendEvent::ToolResult { name, success, preview } => {
                let icon = if *success { "✓" } else { "✗" };
                self.add_tool_activity(&format!("{} {}", icon, name));
                if !success {
                    self.add_message("error", &format!("{} {} failed: {}", icon, name, truncate_str(preview, 100)));
                }
            }
            BackendEvent::TurnComplete { answer } => {
                self.agent_busy = false;
                self.status = "Ready".to_string();
                self.add_message("assistant", answer);
            }
            BackendEvent::Error { message } => {
                self.agent_busy = false;
                self.status = "Ready".to_string();
                self.add_message("error", message);
            }
            BackendEvent::ApprovalRequired { request_id, tool_name, args_display } => {
                self.pending_approval = Some(PendingApproval {
                    request_id: request_id.clone(),
                    tool_name: tool_name.clone(),
                    args_display: args_display.clone(),
                });
            }
            BackendEvent::ModelChanged { model } => {
                self.model_name = model.clone();
                self.selected_model = self.available_models.iter()
                    .position(|m| m == model)
                    .unwrap_or(self.selected_model);
            }
            BackendEvent::StatusUpdate { busy, message } => {
                self.agent_busy = *busy;
                self.status = message.clone();
            }
            BackendEvent::Info { message } => {
                self.add_message("assistant", message);
            }
            BackendEvent::ThinkingText { content } => {
                // In TUI, just show thinking as dimmed assistant text
                self.add_message("assistant", &format!("[thinking] {}", truncate_str(content, 200)));
            }
            BackendEvent::Snapshot(_) => {
                // Full snapshot updates handled separately via from_snapshot
            }
            BackendEvent::DebugLog { .. } => {
                // Debug logs are for GUI only
            }
            BackendEvent::SessionLoaded(session) => {
                self.viewing_session = Some(*session.clone());
            }
            BackendEvent::SessionsUpdated(sessions) => {
                self.sessions = sessions.clone();
                if self.sessions.is_empty() {
                    self.selected_session = 0;
                } else if self.selected_session >= self.sessions.len() {
                    self.selected_session = self.sessions.len() - 1;
                }
            }
            BackendEvent::FlowsListed(flows) => {
                self.flows = flows.clone();
                if self.flows.is_empty() {
                    self.selected_flow = 0;
                } else if self.selected_flow >= self.flows.len() {
                    self.selected_flow = self.flows.len() - 1;
                }
            }
            BackendEvent::FlowLoaded(_) => {
                // TUI doesn't open the flow editor (GUI only)
            }
            BackendEvent::FlowLogsLoaded(_) => {
                // Flow logs displayed in GUI only
            }
            BackendEvent::FlowRunningCount { .. } => {
                // Running flow count displayed in GUI only
            }
            BackendEvent::FlowTemplatesListed(_) => {
                // Flow templates displayed in GUI only
            }
            BackendEvent::IntegrationsUpdated(_) => {
                // Integration updates displayed in GUI only
            }
            BackendEvent::EventsLogUpdated(_) => {
                // Events log displayed in GUI only
            }
            BackendEvent::PacksListed(packs) => {
                self.remote_packs = packs.clone();
                self.packs_loading = false;
                self.selected_pack = 0;
                self.packs_message = None;
            }
            BackendEvent::PackInstalled { slug } => {
                self.packs_loading = false;
                self.packs_message = Some(format!("Installed '{}'", slug));
                // Mark as installed in local list
                for p in &mut self.remote_packs {
                    if p.slug == *slug {
                        p.installed = true;
                    }
                }
            }
            BackendEvent::PackError { message } => {
                self.packs_loading = false;
                self.packs_message = Some(format!("Error: {}", message));
            }
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn add_tool_activity(&mut self, activity: &str) {
        self.tool_activity.push(activity.to_string());
        if self.tool_activity.len() > 20 {
            self.tool_activity.remove(0);
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

/// Handle a key event. Returns Some(FrontendCommand) if input produces a command.
pub fn handle_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    // Global keys
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.should_quit = true;
        return None;
    }

    match state.active_view {
        ActiveView::Chat => handle_chat_key(state, key),
        ActiveView::Skills => { handle_skills_key(state, key); None }
        ActiveView::Graph => { handle_graph_key(state, key); None }
        ActiveView::Personas => { handle_personas_key(state, key); None }
        ActiveView::Memory => { handle_memory_key(state, key); None }
        ActiveView::Data => handle_data_key(state, key),
        ActiveView::Scheduling => handle_scheduling_key(state, key),
        ActiveView::ApiKeys => handle_api_keys_key(state, key),
        ActiveView::Packs => handle_packs_key(state, key),
        ActiveView::Settings => handle_settings_key(state, key),
    }
}

fn handle_chat_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    // Handle approval prompt first
    if state.pending_approval.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(approval) = state.pending_approval.take() {
                    return Some(FrontendCommand::ApprovalResponse {
                        request_id: approval.request_id,
                        approved: true,
                    });
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                if let Some(approval) = state.pending_approval.take() {
                    return Some(FrontendCommand::ApprovalResponse {
                        request_id: approval.request_id,
                        approved: false,
                    });
                }
            }
            _ => {}
        }
        return None;
    }

    match key.code {
        KeyCode::Tab => { state.active_view = state.active_view.next(); None }
        KeyCode::PageUp => {
            state.chat_scroll_up = state.chat_scroll_up.saturating_add(10);
            None
        }
        KeyCode::PageDown => {
            state.chat_scroll_up = state.chat_scroll_up.saturating_sub(10);
            None
        }
        KeyCode::Up if state.agent_busy || state.input.is_empty() => {
            state.chat_scroll_up = state.chat_scroll_up.saturating_add(1);
            None
        }
        KeyCode::Down if state.agent_busy || state.input.is_empty() => {
            state.chat_scroll_up = state.chat_scroll_up.saturating_sub(1);
            None
        }
        KeyCode::Enter => {
            if state.agent_busy || state.input.is_empty() { return None; }
            let input = state.input.clone();
            state.input.clear();
            state.input_cursor = 0;
            state.chat_scroll_up = 0; // snap to bottom on send

            // Add user message to TUI immediately
            state.add_message("user", &input);
            state.agent_busy = true;
            state.status = "Agent thinking...".to_string();

            Some(FrontendCommand::SendMessage { content: input })
        }
        KeyCode::Char(c) => {
            state.input.insert(state.input_cursor, c);
            state.input_cursor += 1;
            None
        }
        KeyCode::Backspace => {
            if state.input_cursor > 0 {
                state.input_cursor -= 1;
                state.input.remove(state.input_cursor);
            }
            None
        }
        KeyCode::Left => { state.input_cursor = state.input_cursor.saturating_sub(1); None }
        KeyCode::Right => { state.input_cursor = (state.input_cursor + 1).min(state.input.len()); None }
        KeyCode::Home => { state.input_cursor = 0; None }
        KeyCode::End => { state.input_cursor = state.input.len(); None }
        _ => None,
    }
}

fn handle_skills_key(state: &mut TuiState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => state.active_view = state.active_view.next(),
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected_skill = state.selected_skill.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.skill_names.is_empty() {
                state.selected_skill = (state.selected_skill + 1).min(state.skill_names.len() - 1);
            }
        }
        KeyCode::Char('o') => {
            let _ = open::that(&state.skills_dir);
        }
        _ => {}
    }
}

fn handle_personas_key(state: &mut TuiState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => state.active_view = state.active_view.next(),
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected_persona = state.selected_persona.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.personas.is_empty() {
                state.selected_persona = (state.selected_persona + 1).min(state.personas.len() - 1);
            }
        }
        KeyCode::Char('o') => {
            let _ = open::that(&state.agents_dir);
        }
        _ => {}
    }
}

fn handle_graph_key(state: &mut TuiState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => state.active_view = state.active_view.next(),
        KeyCode::Up | KeyCode::Char('k') => state.graph_viewport.pan(0.0, -2.0),
        KeyCode::Down | KeyCode::Char('j') => state.graph_viewport.pan(0.0, 2.0),
        KeyCode::Left | KeyCode::Char('h') => state.graph_viewport.pan(-2.0, 0.0),
        KeyCode::Right | KeyCode::Char('l') => state.graph_viewport.pan(2.0, 0.0),
        KeyCode::Char('+') | KeyCode::Char('=') => state.graph_viewport.zoom_in(),
        KeyCode::Char('-') => state.graph_viewport.zoom_out(),
        KeyCode::Char('n') => state.graph_viewport.select_next(state.skill_graph.nodes.len()),
        KeyCode::Char('p') => state.graph_viewport.select_prev(state.skill_graph.nodes.len()),
        _ => {}
    }
}

fn handle_memory_key(state: &mut TuiState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => state.active_view = state.active_view.next(),
        _ => {}
    }
}

fn handle_api_keys_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    match state.api_key_input_mode {
        ApiKeyInputMode::Normal => {
            match key.code {
                KeyCode::Tab => state.active_view = state.active_view.next(),
                KeyCode::Up | KeyCode::Char('k') => {
                    state.selected_api_key = state.selected_api_key.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !state.api_keys.is_empty() {
                        state.selected_api_key = (state.selected_api_key + 1).min(state.api_keys.len() - 1);
                    }
                }
                KeyCode::Char('a') => {
                    state.api_key_input_mode = ApiKeyInputMode::EnteringName;
                    state.api_key_name_input.clear();
                    state.api_key_value_input.clear();
                }
                KeyCode::Char('d') => {
                    if state.selected_api_key < state.api_keys.len() {
                        let name = state.api_keys[state.selected_api_key].0.clone();
                        if state.selected_api_key > 0 {
                            state.selected_api_key -= 1;
                        }
                        return Some(FrontendCommand::ApiKeyDelete { name });
                    }
                }
                KeyCode::Char('p') => {
                    state.active_view = ActiveView::Packs;
                    return Some(FrontendCommand::PacksList);
                }
                _ => {}
            }
        }
        ApiKeyInputMode::EnteringName => {
            match key.code {
                KeyCode::Esc => {
                    state.api_key_input_mode = ApiKeyInputMode::Normal;
                }
                KeyCode::Enter => {
                    if !state.api_key_name_input.is_empty() {
                        state.api_key_input_mode = ApiKeyInputMode::EnteringKey;
                    }
                }
                KeyCode::Char(c) => {
                    state.api_key_name_input.push(c.to_ascii_uppercase());
                }
                KeyCode::Backspace => {
                    state.api_key_name_input.pop();
                }
                _ => {}
            }
        }
        ApiKeyInputMode::EnteringKey => {
            match key.code {
                KeyCode::Esc => {
                    state.api_key_input_mode = ApiKeyInputMode::Normal;
                }
                KeyCode::Enter => {
                    if !state.api_key_value_input.is_empty() {
                        let name = state.api_key_name_input.clone();
                        let key_val = state.api_key_value_input.clone();
                        state.api_key_input_mode = ApiKeyInputMode::Normal;
                        state.api_key_name_input.clear();
                        state.api_key_value_input.clear();
                        return Some(FrontendCommand::ApiKeyAdd { name, key: key_val });
                    }
                }
                KeyCode::Char(c) => {
                    state.api_key_value_input.push(c);
                }
                KeyCode::Backspace => {
                    state.api_key_value_input.pop();
                }
                _ => {}
            }
        }
    }
    None
}

fn handle_data_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    match key.code {
        KeyCode::Tab => { state.active_view = state.active_view.next(); }
        KeyCode::Char('h') | KeyCode::Left => {
            state.data_focus = match state.data_focus {
                DataFocus::Detail => DataFocus::SessionList,
                DataFocus::SessionList => DataFocus::Sidebar,
                DataFocus::Sidebar => DataFocus::Sidebar,
            };
        }
        KeyCode::Char('l') | KeyCode::Right => {
            state.data_focus = match state.data_focus {
                DataFocus::Sidebar => DataFocus::SessionList,
                DataFocus::SessionList => DataFocus::Detail,
                DataFocus::Detail => DataFocus::Detail,
            };
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.data_focus == DataFocus::SessionList {
                state.selected_session = state.selected_session.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.data_focus == DataFocus::SessionList && !state.sessions.is_empty() {
                state.selected_session = (state.selected_session + 1).min(state.sessions.len() - 1);
            }
        }
        KeyCode::Enter => {
            if state.data_focus == DataFocus::SessionList && state.selected_session < state.sessions.len() {
                let id = state.sessions[state.selected_session].id.clone();
                state.data_focus = DataFocus::Detail;
                return Some(FrontendCommand::LoadSession { session_id: id });
            }
        }
        KeyCode::Char('d') => {
            if state.data_focus == DataFocus::SessionList && state.selected_session < state.sessions.len() {
                let id = state.sessions[state.selected_session].id.clone();
                state.viewing_session = None;
                return Some(FrontendCommand::DeleteSession { session_id: id });
            }
        }
        _ => {}
    }
    None
}

fn handle_scheduling_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    match key.code {
        KeyCode::Tab => { state.active_view = state.active_view.next(); }
        KeyCode::Char('h') | KeyCode::Left => {
            state.flows_focus = FlowsFocus::List;
        }
        KeyCode::Char('l') | KeyCode::Right => {
            state.flows_focus = FlowsFocus::Detail;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.flows_focus == FlowsFocus::List {
                state.selected_flow = state.selected_flow.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.flows_focus == FlowsFocus::List && !state.flows.is_empty() {
                state.selected_flow = (state.selected_flow + 1).min(state.flows.len() - 1);
            }
        }
        KeyCode::Char('d') => {
            if state.flows_focus == FlowsFocus::List && state.selected_flow < state.flows.len() {
                let id = state.flows[state.selected_flow].id.clone();
                return Some(FrontendCommand::FlowDelete { flow_id: id });
            }
        }
        _ => {}
    }
    None
}

fn handle_packs_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    match key.code {
        KeyCode::Tab => { state.active_view = state.active_view.next(); }
        KeyCode::Char('h') | KeyCode::Left => {
            state.packs_focus = PacksFocus::List;
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if !state.remote_packs.is_empty() {
                state.packs_focus = PacksFocus::Detail;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.packs_focus == PacksFocus::List {
                state.selected_pack = state.selected_pack.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.packs_focus == PacksFocus::List {
                let filtered = filtered_packs(state);
                if state.selected_pack + 1 < filtered.len() {
                    state.selected_pack += 1;
                }
            }
        }
        KeyCode::Char('r') => {
            // Refresh: fetch packs from server
            state.packs_loading = true;
            state.packs_message = Some("Fetching packs...".to_string());
            return Some(FrontendCommand::PacksList);
        }
        KeyCode::Enter | KeyCode::Char('i') => {
            // Install selected pack
            let filtered = filtered_packs(state);
            if let Some(pack) = filtered.get(state.selected_pack) {
                if !pack.installed {
                    let slug = pack.slug.clone();
                    state.packs_loading = true;
                    state.packs_message = Some(format!("Installing '{}'...", slug));
                    return Some(FrontendCommand::PackInstall { slug });
                }
            }
        }
        KeyCode::Char('d') => {
            // Uninstall selected pack
            let filtered = filtered_packs(state);
            if let Some(pack) = filtered.get(state.selected_pack) {
                if pack.installed {
                    let slug = pack.slug.clone();
                    state.packs_message = Some(format!("Uninstalling '{}'...", slug));
                    // Mark as uninstalled locally
                    for p in &mut state.remote_packs {
                        if p.slug == slug {
                            p.installed = false;
                        }
                    }
                    return Some(FrontendCommand::PackUninstall { slug });
                }
            }
        }
        KeyCode::Char('/') => {
            // Start search (just clear to let user type)
            state.packs_search.clear();
            state.selected_pack = 0;
        }
        KeyCode::Backspace => {
            state.packs_search.pop();
            state.selected_pack = 0;
        }
        KeyCode::Esc => {
            if !state.packs_search.is_empty() {
                state.packs_search.clear();
                state.selected_pack = 0;
            }
        }
        KeyCode::Char(c) if !c.is_ascii_control() && state.packs_focus == PacksFocus::List => {
            // Type-to-search
            if c != 'h' && c != 'l' && c != 'k' && c != 'j' && c != 'r' && c != 'i' && c != 'd' {
                state.packs_search.push(c);
                state.selected_pack = 0;
            }
        }
        _ => {}
    }
    None
}

fn filtered_packs(state: &TuiState) -> Vec<starkbot_api::types::PackInfo> {
    if state.packs_search.is_empty() {
        return state.remote_packs.clone();
    }
    let q = state.packs_search.to_lowercase();
    state.remote_packs.iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.slug.to_lowercase().contains(&q)
            || p.description.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

fn handle_settings_key(state: &mut TuiState, key: KeyEvent) -> Option<FrontendCommand> {
    match key.code {
        KeyCode::Tab => state.active_view = state.active_view.next(),
        KeyCode::Char('h') | KeyCode::Left if state.settings_focus == SettingsFocus::Content => {
            state.settings_focus = SettingsFocus::Sidebar;
        }
        KeyCode::Char('l') | KeyCode::Right if state.settings_focus == SettingsFocus::Sidebar => {
            state.settings_focus = SettingsFocus::Content;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_focus == SettingsFocus::Content {
                if state.settings_section == SettingsSection::AgentModel {
                    state.selected_model = state.selected_model.saturating_sub(1);
                }
            } else {
                let sections = SettingsSection::all();
                if let Some(idx) = sections.iter().position(|s| *s == state.settings_section) {
                    if idx > 0 { state.settings_section = sections[idx - 1]; }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.settings_focus == SettingsFocus::Content {
                if state.settings_section == SettingsSection::AgentModel && !state.available_models.is_empty() {
                    state.selected_model = (state.selected_model + 1).min(state.available_models.len() - 1);
                }
            } else {
                let sections = SettingsSection::all();
                if let Some(idx) = sections.iter().position(|s| *s == state.settings_section) {
                    if idx + 1 < sections.len() { state.settings_section = sections[idx + 1]; }
                }
            }
        }
        KeyCode::Enter => {
            if state.settings_focus == SettingsFocus::Content {
                if state.selected_model < state.available_models.len() {
                    let new_model = &state.available_models[state.selected_model];
                    if *new_model != state.model_name {
                        return Some(FrontendCommand::SwitchModel {
                            model: new_model.clone(),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    None
}

/// Draw the full TUI frame.
pub fn draw(frame: &mut ratatui::Frame, state: &TuiState) {
    // Clear the entire frame to prevent ghosting artifacts from previous views
    frame.render_widget(Clear, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Tab bar
            Constraint::Min(10),   // Main content
            Constraint::Length(1),  // Status bar
        ])
        .split(frame.area());

    // Tab bar
    let titles: Vec<Line> = ActiveView::titles().iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .select(state.active_view.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Cyan).bold())
        .divider(" │ ");
    frame.render_widget(tabs, chunks[0]);

    // Main content
    match state.active_view {
        ActiveView::Chat => draw_chat(frame, state, chunks[1]),
        ActiveView::Skills => draw_skills(frame, state, chunks[1]),
        ActiveView::Graph => draw_graph(frame, state, chunks[1]),
        ActiveView::Personas => draw_personas(frame, state, chunks[1]),
        ActiveView::Memory => draw_memory(frame, state, chunks[1]),
        ActiveView::Data => draw_data(frame, state, chunks[1]),
        ActiveView::Scheduling => draw_scheduling(frame, state, chunks[1]),
        ActiveView::ApiKeys => draw_api_keys(frame, state, chunks[1]),
        ActiveView::Packs => draw_packs(frame, state, chunks[1]),
        ActiveView::Settings => draw_settings(frame, state, chunks[1]),
    }

    // Status bar
    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", if state.agent_busy { "⟳ Agent thinking..." } else { "Ready" }),
            Style::default().fg(if state.agent_busy { Color::Yellow } else { Color::Green }),
        ),
        Span::raw(" │ "),
        Span::styled(format!("Persona: {}", state.persona_name), Style::default().fg(Color::Cyan)),
        Span::raw(" │ "),
        Span::styled(format!("Model: {}", state.model_name), Style::default().fg(Color::DarkGray)),
        Span::raw(" │ "),
        Span::styled("Tab: switch │ PgUp/Dn: scroll │ Ctrl+C: quit", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[2]);
}

/// Render inline markdown: **bold**, `code`, and plain text.
fn render_inline_markdown<'a>(text: &'a str, spans: &mut Vec<Span<'a>>) {
    let mut remaining = text;
    while !remaining.is_empty() {
        // Check for **bold**
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                spans.push(Span::raw(&remaining[..start]));
            }
            let after_start = &remaining[start + 2..];
            if let Some(end) = after_start.find("**") {
                spans.push(Span::styled(
                    &after_start[..end],
                    Style::default().fg(Color::White).bold(),
                ));
                remaining = &after_start[end + 2..];
            } else {
                spans.push(Span::raw(&remaining[start..]));
                return;
            }
        }
        // Check for `code`
        else if let Some(start) = remaining.find('`') {
            if start > 0 {
                spans.push(Span::raw(&remaining[..start]));
            }
            let after_start = &remaining[start + 1..];
            if let Some(end) = after_start.find('`') {
                spans.push(Span::styled(
                    &after_start[..end],
                    Style::default().fg(Color::Yellow),
                ));
                remaining = &after_start[end + 1..];
            } else {
                spans.push(Span::raw(&remaining[start..]));
                return;
            }
        }
        // No more inline markdown
        else {
            spans.push(Span::raw(remaining));
            return;
        }
    }
}

fn draw_chat(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let input_height = if state.pending_approval.is_some() { 6 } else { 3 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),                    // Messages
            Constraint::Length(input_height),      // Input / Approval
            Constraint::Length(3),                 // Tool activity
        ])
        .split(area);

    // Messages — render multi-line content with markdown styling
    let mut lines: Vec<Line> = vec![];
    let mut in_code_block = false;

    for msg in &state.messages {
        let (prefix, color) = match msg.role.as_str() {
            "user" => ("[you]", Color::Green),
            "assistant" => ("[agent]", Color::Cyan),
            "tool" => ("[tool]", Color::Yellow),
            "error" => ("[error]", Color::Red),
            _ => ("[?]", Color::Gray),
        };

        let content_lines: Vec<&str> = msg.content.split('\n').collect();
        for (i, line_text) in content_lines.iter().enumerate() {
            let mut spans: Vec<Span> = vec![];

            if i == 0 {
                spans.push(Span::styled(format!("{} ", prefix), Style::default().fg(color).bold()));
            } else {
                spans.push(Span::raw("       "));
            }

            // Code block toggle
            if line_text.starts_with("```") {
                in_code_block = !in_code_block;
                if in_code_block {
                    let lang = line_text.trim_start_matches('`').trim();
                    if !lang.is_empty() {
                        spans.push(Span::styled(format!("─── {} ", lang), Style::default().fg(Color::DarkGray)));
                    } else {
                        spans.push(Span::styled("───", Style::default().fg(Color::DarkGray)));
                    }
                } else {
                    spans.push(Span::styled("───", Style::default().fg(Color::DarkGray)));
                }
                lines.push(Line::from(spans));
                continue;
            }

            if in_code_block {
                spans.push(Span::styled(
                    format!("  {}", line_text),
                    Style::default().fg(Color::Green),
                ));
                lines.push(Line::from(spans));
                continue;
            }

            // Markdown rendering
            if line_text.starts_with("## ") {
                spans.push(Span::styled(
                    line_text.trim_start_matches("## "),
                    Style::default().fg(Color::White).bold(),
                ));
            } else if line_text.starts_with("### ") {
                spans.push(Span::styled(
                    line_text.trim_start_matches("### "),
                    Style::default().fg(Color::Cyan).bold(),
                ));
            } else if line_text.starts_with("# ") {
                spans.push(Span::styled(
                    line_text.trim_start_matches("# "),
                    Style::default().fg(Color::White).bold(),
                ));
            } else if line_text.starts_with("- ") || line_text.starts_with("  - ") || line_text.starts_with("    - ") {
                let indent = line_text.len() - line_text.trim_start().len();
                let bullet_text = line_text.trim_start_matches(|c: char| c == ' ' || c == '-').trim_start();
                spans.push(Span::raw(" ".repeat(indent)));
                spans.push(Span::styled("• ", Style::default().fg(Color::DarkGray)));
                render_inline_markdown(bullet_text, &mut spans);
            } else if line_text.trim().is_empty() {
                spans.push(Span::raw(""));
            } else if line_text.starts_with(|c: char| c.is_ascii_digit()) && line_text.contains(". ") {
                if let Some(pos) = line_text.find(". ") {
                    let num = &line_text[..pos + 2];
                    let rest = &line_text[pos + 2..];
                    spans.push(Span::styled(num, Style::default().fg(Color::DarkGray)));
                    render_inline_markdown(rest, &mut spans);
                } else {
                    render_inline_markdown(line_text, &mut spans);
                }
            } else {
                render_inline_markdown(line_text, &mut spans);
            }

            lines.push(Line::from(spans));
        }
        lines.push(Line::from(""));
    }

    // Calculate scroll
    let content_width = chunks[0].width.saturating_sub(2) as usize;
    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let mut wrapped_total: usize = 0;
    for line in &lines {
        let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
        if content_width > 0 && line_width > content_width {
            wrapped_total += (line_width + content_width - 1) / content_width;
        } else {
            wrapped_total += 1;
        }
    }

    let max_scroll = wrapped_total.saturating_sub(visible_height) as u16;
    let clamped_up = state.chat_scroll_up.min(max_scroll);
    let scroll_offset = max_scroll.saturating_sub(clamped_up);

    let scroll_indicator = if clamped_up > 0 {
        format!(" Messages [↑{}] ", clamped_up)
    } else {
        " Messages ".to_string()
    };
    let messages = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(scroll_indicator))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(messages, chunks[0]);

    // Input (or approval prompt)
    if let Some(ref approval) = state.pending_approval {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(&approval.tool_name, Style::default().fg(Color::White).bold()),
            ]),
        ];
        for part in approval.args_display.split('\n') {
            let trimmed = if part.len() > 70 { format!("{:.70}...", part) } else { part.to_string() };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(trimmed, Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("                    ", Style::default()),
            Span::styled("[Y]", Style::default().fg(Color::Green).bold()),
            Span::styled("es  ", Style::default().fg(Color::Green)),
            Span::styled("[N]", Style::default().fg(Color::Red).bold()),
            Span::styled("o", Style::default().fg(Color::Red)),
        ]));
        let input = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL)
                .title(Span::styled(" ⚡ Approve Tool ", Style::default().fg(Color::Yellow).bold()))
                .border_style(Style::default().fg(Color::Yellow)));
        frame.render_widget(input, chunks[1]);
    } else {
        let input_display = if state.agent_busy {
            " Agent is thinking...".to_string()
        } else {
            format!(" > {}", state.input)
        };
        let input = Paragraph::new(input_display)
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .style(if state.agent_busy {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            });
        frame.render_widget(input, chunks[1]);
    }

    // Tool activity
    let activity: Vec<Line> = state.tool_activity.iter().rev().take(2)
        .map(|a| Line::from(Span::styled(format!(" {}", a), Style::default().fg(Color::DarkGray))))
        .collect();
    let tool_bar = Paragraph::new(activity)
        .block(Block::default().borders(Borders::ALL).title(" Tools "));
    frame.render_widget(tool_bar, chunks[2]);

    // Set cursor
    if !state.agent_busy {
        frame.set_cursor_position((
            chunks[1].x + 3 + state.input_cursor as u16,
            chunks[1].y + 1,
        ));
    }
}

fn draw_skills(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<Line> = state.skill_names.iter().enumerate().map(|(i, name)| {
        let style = if i == state.selected_skill {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        };
        let marker = if i == state.selected_skill { "▸ " } else { "  " };
        Line::from(Span::styled(format!("{}{}", marker, name), style))
    }).collect();
    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Skills ")
            .title_bottom(Line::from(" o: Open folder ").right_aligned()));
    frame.render_widget(list, chunks[0]);

    let detail_text = if state.skills.is_empty() {
        "No skills loaded".to_string()
    } else if state.selected_skill < state.skills.len() {
        let skill = &state.skills[state.selected_skill];
        let mut text = format!("# {}\n", skill.name);
        if !skill.description.is_empty() {
            text.push_str(&format!("{}\n", skill.description));
        }
        text.push('\n');
        if !skill.version.is_empty() {
            text.push_str(&format!("Version: {}\n", skill.version));
        }
        if !skill.tags.is_empty() {
            text.push_str(&format!("Tags: {}\n", skill.tags.join(", ")));
        }
        if !skill.requires_tools.is_empty() {
            text.push_str(&format!("Tools: {}\n", skill.requires_tools.join(", ")));
        }
        text.push_str("\n─────────────────────────────────\n\n");
        text.push_str(&skill.content);
        text
    } else {
        String::new()
    };
    let detail_title = if state.selected_skill < state.skills.len() {
        format!(" {} ", state.skills[state.selected_skill].name)
    } else {
        " Detail ".to_string()
    };
    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(detail_title))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[1]);
}

fn draw_personas(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let items: Vec<Line> = state.personas.iter().enumerate().map(|(i, p)| {
        let style = if i == state.selected_persona {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default()
        };
        let marker = if i == state.selected_persona { "▸ " } else { "  " };
        let label = format!("{}{} {}", marker, p.emoji, p.label);
        Line::from(Span::styled(label, style))
    }).collect();
    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Personas ")
            .title_bottom(Line::from(" o: Open folder ").right_aligned()));
    frame.render_widget(list, chunks[0]);

    let detail_text = if state.personas.is_empty() {
        "No personas loaded".to_string()
    } else if state.selected_persona < state.personas.len() {
        let p = &state.personas[state.selected_persona];
        let mut text = format!("{} {}\n", p.emoji, p.label);
        if !p.description.is_empty() {
            text.push_str(&format!("{}\n", p.description));
        }
        text.push('\n');
        text.push_str(&format!("Version: {}\n", p.version));
        text.push_str(&format!("Enabled: {}\n", if p.enabled { "yes" } else { "no" }));
        if !p.tool_groups.is_empty() {
            text.push_str(&format!("Tool groups: {}\n", p.tool_groups.join(", ")));
        }
        if !p.skill_tags.is_empty() {
            text.push_str(&format!("Skill tags: {}\n", p.skill_tags.join(", ")));
        }
        if !p.explicit_skills.is_empty() {
            text.push_str(&format!("Skills: {}\n", p.explicit_skills.join(", ")));
        }
        text.push_str("\n─────────────────────────────────\n\n");
        let preview: String = p.system_prompt.chars().take(500).collect();
        text.push_str(&preview);
        if p.system_prompt.len() > 500 {
            text.push_str("\n...");
        }
        text
    } else {
        String::new()
    };
    let detail_title = if state.selected_persona < state.personas.len() {
        format!(" {} ", state.personas[state.selected_persona].label)
    } else {
        " Detail ".to_string()
    };
    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(detail_title))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[1]);
}

fn draw_graph(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Skill Graph ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(GraphWidget::new(&state.skill_graph, &state.graph_viewport), inner);
}

fn draw_memory(frame: &mut ratatui::Frame, _state: &TuiState, area: Rect) {
    let placeholder = Paragraph::new(" Memory browser - coming soon\n\n Use the agent to search memories via `memory_search` tool.")
        .block(Block::default().borders(Borders::ALL).title(" Memory "))
        .wrap(Wrap { trim: false });
    frame.render_widget(placeholder, area);
}


fn draw_data(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),     // Pill sidebar
            Constraint::Percentage(35), // Session list
            Constraint::Percentage(65), // Detail
        ])
        .split(area);

    // Pill sidebar
    let sidebar_focused = state.data_focus == DataFocus::Sidebar;
    let mut pill_lines: Vec<Line> = vec![Line::from("")];
    let style = if sidebar_focused {
        Style::default().fg(Color::Black).bg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Cyan).bold()
    };
    pill_lines.push(Line::from(Span::styled(" Chat Sessions ", style)));
    pill_lines.push(Line::from(""));
    let sidebar_border = if sidebar_focused { Color::Cyan } else { Color::DarkGray };
    let sidebar = Paragraph::new(pill_lines)
        .block(Block::default().borders(Borders::ALL).title(" Data ")
            .border_style(Style::default().fg(sidebar_border)));
    frame.render_widget(sidebar, chunks[0]);

    // Session list
    let list_focused = state.data_focus == DataFocus::SessionList;
    let mut items: Vec<Line> = vec![];
    if state.sessions.is_empty() {
        items.push(Line::from(Span::styled("  No saved sessions", Style::default().fg(Color::DarkGray))));
        items.push(Line::from(Span::styled("  Chat to create one", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, s) in state.sessions.iter().enumerate() {
            let is_selected = i == state.selected_session;
            let style = if is_selected && list_focused {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };
            let marker = if is_selected { "▸ " } else { "  " };
            let date_str = s.created_at.get(..10).unwrap_or(&s.created_at);
            items.push(Line::from(vec![
                Span::styled(format!("{}{}", marker, truncate_str(&s.title, 30)), style),
            ]));
            items.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{} | {} msgs | {}", s.persona, s.message_count, date_str), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    let list_border = if list_focused { Color::Cyan } else { Color::DarkGray };
    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Sessions ")
            .title_bottom(Line::from(" Enter: view | d: delete ").right_aligned())
            .border_style(Style::default().fg(list_border)));
    frame.render_widget(list, chunks[1]);

    // Detail
    let detail_focused = state.data_focus == DataFocus::Detail;
    let mut detail_lines: Vec<Line> = vec![];
    if let Some(ref session) = state.viewing_session {
        for msg in &session.messages {
            let (prefix, color) = match msg.role.as_str() {
                "user" => ("[you]", Color::Green),
                "assistant" => ("[agent]", Color::Cyan),
                _ => ("[?]", Color::Gray),
            };
            let content_lines: Vec<&str> = msg.content.split('\n').collect();
            for (i, line_text) in content_lines.iter().enumerate() {
                if i == 0 {
                    detail_lines.push(Line::from(vec![
                        Span::styled(format!("{} ", prefix), Style::default().fg(color).bold()),
                        Span::raw(*line_text),
                    ]));
                } else {
                    detail_lines.push(Line::from(vec![
                        Span::raw("       "),
                        Span::raw(*line_text),
                    ]));
                }
            }
            detail_lines.push(Line::from(""));
        }
    } else {
        detail_lines.push(Line::from(Span::styled("  Select a session to view", Style::default().fg(Color::DarkGray))));
    }
    let detail_title = if let Some(ref s) = state.viewing_session {
        format!(" {} ", truncate_str(&s.title, 40))
    } else {
        " Detail ".to_string()
    };
    let detail_border = if detail_focused { Color::Cyan } else { Color::DarkGray };
    let detail = Paragraph::new(detail_lines)
        .block(Block::default().borders(Borders::ALL).title(detail_title)
            .border_style(Style::default().fg(detail_border)))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[2]);
}

fn draw_scheduling(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Flow list
    let list_focused = state.flows_focus == FlowsFocus::List;
    let mut items: Vec<Line> = vec![];
    if state.flows.is_empty() {
        items.push(Line::from(Span::styled("  No flows", Style::default().fg(Color::DarkGray))));
        items.push(Line::from(Span::styled("  Create via GUI", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, f) in state.flows.iter().enumerate() {
            let is_selected = i == state.selected_flow;
            let style = if is_selected && list_focused {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };
            let marker = if is_selected { "▸ " } else { "  " };
            items.push(Line::from(Span::styled(format!("{}{}", marker, f.name), style)));
            items.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{} nodes | {}", f.node_count, f.updated_at.get(..10).unwrap_or(&f.updated_at)),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }
    let list_border = if list_focused { Color::Cyan } else { Color::DarkGray };
    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Flows ")
            .title_bottom(Line::from(" d: delete ").right_aligned())
            .border_style(Style::default().fg(list_border)));
    frame.render_widget(list, chunks[0]);

    // Detail
    let detail_focused = state.flows_focus == FlowsFocus::Detail;
    let mut detail_lines: Vec<Line> = vec![];
    if state.selected_flow < state.flows.len() {
        let f = &state.flows[state.selected_flow];
        detail_lines.push(Line::from(Span::styled(format!(" Flow: {}", f.name), Style::default().fg(Color::White).bold())));
        detail_lines.push(Line::from(Span::styled(format!(" Nodes: {}", f.node_count), Style::default().fg(Color::DarkGray))));
        detail_lines.push(Line::from(Span::styled(format!(" Created: {}", f.created_at.get(..10).unwrap_or(&f.created_at)), Style::default().fg(Color::DarkGray))));
        detail_lines.push(Line::from(Span::styled(format!(" Updated: {}", f.updated_at.get(..10).unwrap_or(&f.updated_at)), Style::default().fg(Color::DarkGray))));
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled("  Edit flows in the GUI", Style::default().fg(Color::DarkGray))));
    } else {
        detail_lines.push(Line::from(Span::styled("  Select a flow to view", Style::default().fg(Color::DarkGray))));
    }
    let detail_border = if detail_focused { Color::Cyan } else { Color::DarkGray };
    let detail = Paragraph::new(detail_lines)
        .block(Block::default().borders(Borders::ALL).title(" Details ")
            .border_style(Style::default().fg(detail_border)))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[1]);
}

fn draw_api_keys(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let mut items: Vec<Line> = vec![];
    if state.api_keys.is_empty() {
        items.push(Line::from(Span::styled(
            "  No API keys configured", Style::default().fg(Color::DarkGray),
        )));
        items.push(Line::from(Span::styled(
            "  Press 'a' to add one", Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, (name, masked, _)) in state.api_keys.iter().enumerate() {
            let style = if i == state.selected_api_key {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };
            let marker = if i == state.selected_api_key { "▸ " } else { "  " };
            items.push(Line::from(vec![
                Span::styled(format!("{}{}", marker, name), style),
                Span::styled(format!("  {}", masked), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    let list = Paragraph::new(items)
        .block(Block::default().borders(Borders::ALL).title(" API Keys "));
    frame.render_widget(list, chunks[0]);

    let mut detail_lines: Vec<Line> = vec![];

    match state.api_key_input_mode {
        ApiKeyInputMode::Normal => {
            if state.selected_api_key < state.api_keys.len() {
                let (name, masked, updated) = &state.api_keys[state.selected_api_key];
                detail_lines.push(Line::from(Span::styled(format!(" Service: {}", name), Style::default().fg(Color::White).bold())));
                detail_lines.push(Line::from(Span::styled(format!(" Key:     {}", masked), Style::default().fg(Color::Yellow))));
                detail_lines.push(Line::from(Span::styled(format!(" Updated: {}", updated), Style::default().fg(Color::DarkGray))));
                detail_lines.push(Line::from(""));
            }
            detail_lines.push(Line::from(Span::styled(" Shortcuts:", Style::default().fg(Color::Cyan))));
            detail_lines.push(Line::from("   a - Add new key"));
            detail_lines.push(Line::from("   d - Delete selected key"));
            detail_lines.push(Line::from("   j/k - Navigate"));
            detail_lines.push(Line::from("   p - Find more integrations"));
            detail_lines.push(Line::from("   Tab - Switch view"));
        }
        ApiKeyInputMode::EnteringName => {
            detail_lines.push(Line::from(Span::styled(" Add API Key", Style::default().fg(Color::Cyan).bold())));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![
                Span::styled(" Service name: ", Style::default().fg(Color::Yellow)),
                Span::styled(&state.api_key_name_input, Style::default().fg(Color::White).bold()),
                Span::styled("_", Style::default().fg(Color::White)),
            ]));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                " Use UPPER_SNAKE_CASE (e.g. CLOUDFLARE_API_TOKEN)", Style::default().fg(Color::DarkGray),
            )));
            detail_lines.push(Line::from(Span::styled(" Enter: next │ Esc: cancel", Style::default().fg(Color::DarkGray))));
        }
        ApiKeyInputMode::EnteringKey => {
            detail_lines.push(Line::from(Span::styled(" Add API Key", Style::default().fg(Color::Cyan).bold())));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![
                Span::styled(" Service: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&state.api_key_name_input),
            ]));
            let display_key = if state.api_key_value_input.len() > 4 {
                format!("{}...", &state.api_key_value_input[..4])
            } else {
                state.api_key_value_input.clone()
            };
            detail_lines.push(Line::from(vec![
                Span::styled(" API key: ", Style::default().fg(Color::Yellow)),
                Span::styled(display_key, Style::default().fg(Color::White).bold()),
                Span::styled("_", Style::default().fg(Color::White)),
            ]));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(" Enter: save │ Esc: cancel", Style::default().fg(Color::DarkGray))));
        }
    }

    let detail = Paragraph::new(detail_lines)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, chunks[1]);
}

fn draw_packs(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let filtered = filtered_packs(state);

    // --- Left: pack list ---
    let list_focused = state.packs_focus == PacksFocus::List;
    let mut items: Vec<Line> = vec![];

    // Search bar
    if !state.packs_search.is_empty() {
        items.push(Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(Color::Yellow)),
            Span::styled(&state.packs_search, Style::default().fg(Color::White).bold()),
        ]));
        items.push(Line::from(""));
    }

    if state.packs_loading && filtered.is_empty() {
        items.push(Line::from(Span::styled("  Loading...", Style::default().fg(Color::Yellow))));
    } else if filtered.is_empty() {
        items.push(Line::from(Span::styled(
            if state.remote_packs.is_empty() {
                "  Press 'r' to fetch packs"
            } else {
                "  No packs match search"
            },
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, pack) in filtered.iter().enumerate() {
            let is_selected = i == state.selected_pack;
            let style = if is_selected && list_focused {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default()
            };
            let marker = if is_selected { "▸ " } else { "  " };
            let status = if pack.installed { " ✓" } else { "" };
            items.push(Line::from(vec![
                Span::styled(format!("{}{}", marker, pack.name), style),
                Span::styled(status, Style::default().fg(Color::Green)),
            ]));
            items.push(Line::from(Span::styled(
                format!("    {}", truncate_str(&pack.description, 40)),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let mut list_title = " Packs ".to_string();
    if !state.remote_packs.is_empty() {
        list_title = format!(" Packs ({}) ", filtered.len());
    }
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(list_title)
        .title_bottom(Line::from(" r: refresh │ /: search │ Esc: clear ").right_aligned())
        .border_style(Style::default().fg(if list_focused { Color::Cyan } else { Color::DarkGray }));
    let list = Paragraph::new(items).block(list_block);
    frame.render_widget(list, chunks[0]);

    // --- Right: detail pane ---
    let detail_focused = state.packs_focus == PacksFocus::Detail;
    let mut detail_lines: Vec<Line> = vec![];

    if let Some(pack) = filtered.get(state.selected_pack) {
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(&pack.name, Style::default().fg(Color::Cyan).bold()),
            if pack.installed {
                Span::styled("  (installed)", Style::default().fg(Color::Green))
            } else {
                Span::styled("  (not installed)", Style::default().fg(Color::DarkGray))
            },
        ]));
        detail_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("slug: {}", pack.slug), Style::default().fg(Color::DarkGray)),
        ]));
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(&pack.description),
        ]));
        if let Some(icon) = &pack.icon {
            detail_lines.push(Line::from(vec![
                Span::raw("  icon: "),
                Span::styled(icon, Style::default().fg(Color::Yellow)),
            ]));
        }
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(""));

        // Action hints
        if pack.installed {
            detail_lines.push(Line::from(Span::styled(
                "  Press 'd' to uninstall",
                Style::default().fg(Color::Red),
            )));
        } else {
            detail_lines.push(Line::from(Span::styled(
                "  Press Enter or 'i' to install",
                Style::default().fg(Color::Green),
            )));
        }
    } else {
        detail_lines.push(Line::from(Span::styled(
            "  Select a pack to view details",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Status message
    if let Some(msg) = &state.packs_message {
        detail_lines.push(Line::from(""));
        let color = if msg.starts_with("Error") { Color::Red } else { Color::Yellow };
        detail_lines.push(Line::from(Span::styled(
            format!("  {}", msg),
            Style::default().fg(color),
        )));
    }

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .title_bottom(Line::from(" i: install │ d: uninstall ").right_aligned())
        .border_style(Style::default().fg(if detail_focused { Color::Cyan } else { Color::DarkGray }));
    let detail = Paragraph::new(detail_lines)
        .wrap(Wrap { trim: false })
        .block(detail_block);
    frame.render_widget(detail, chunks[1]);
}

fn draw_settings(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),     // Pill sidebar
            Constraint::Percentage(40), // Model list
            Constraint::Percentage(60), // Detail
        ])
        .split(area);

    // Pill sidebar
    let sidebar_focused = state.settings_focus == SettingsFocus::Sidebar;
    let mut pill_lines: Vec<Line> = vec![Line::from("")];
    for section in SettingsSection::all() {
        let is_active = *section == state.settings_section;
        let style = if is_active && sidebar_focused {
            Style::default().fg(Color::Black).bg(Color::Cyan).bold()
        } else if is_active {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        pill_lines.push(Line::from(Span::styled(format!(" {} ", section.label()), style)));
        pill_lines.push(Line::from(""));
    }
    let sidebar_border = if sidebar_focused { Color::Cyan } else { Color::DarkGray };
    let sidebar = Paragraph::new(pill_lines)
        .block(Block::default().borders(Borders::ALL).title(" Settings ")
            .border_style(Style::default().fg(sidebar_border)));
    frame.render_widget(sidebar, chunks[0]);

    // Content area
    let content_focused = state.settings_focus == SettingsFocus::Content;
    let list_border = if content_focused { Color::Cyan } else { Color::DarkGray };

    match state.settings_section {
        SettingsSection::AgentModel => {
            let items: Vec<Line> = state.available_models.iter().enumerate().map(|(i, model)| {
                let is_current = *model == state.model_name;
                let is_selected = i == state.selected_model;
                let style = if is_selected && content_focused {
                    Style::default().fg(Color::Cyan).bold()
                } else if is_current {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                let marker = if is_selected && content_focused { "▸ " } else { "  " };
                let current_tag = if is_current { " (active)" } else { "" };
                Line::from(Span::styled(format!("{}{}{}", marker, model, current_tag), style))
            }).collect();
            let list = Paragraph::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Models ")
                    .border_style(Style::default().fg(list_border)));
            frame.render_widget(list, chunks[1]);

            // Detail
            let mut detail_lines: Vec<Line> = vec![];
            if state.selected_model < state.available_models.len() {
                let model = &state.available_models[state.selected_model];
                detail_lines.push(Line::from(Span::styled(
                    format!(" Model: {}", model), Style::default().fg(Color::White).bold(),
                )));
                detail_lines.push(Line::from(""));
                let desc = match model.as_str() {
                    "gpt-5.4" => "Default model. Good balance of speed and capability.",
                    "gpt-5.4-mini" => "Faster and cheaper. Best for simple tasks.",
                    "gpt-5.5" => "Most capable. Best for complex reasoning.",
                    _ => "",
                };
                detail_lines.push(Line::from(Span::styled(format!(" {}", desc), Style::default().fg(Color::DarkGray))));
                detail_lines.push(Line::from(""));
            }
            detail_lines.push(Line::from(Span::styled(" Shortcuts:", Style::default().fg(Color::Cyan))));
            detail_lines.push(Line::from("   h/l - Switch sidebar/content"));
            detail_lines.push(Line::from("   j/k - Navigate"));
            detail_lines.push(Line::from("   Enter - Select model"));
            detail_lines.push(Line::from("   Tab - Switch view"));

            let detail = Paragraph::new(detail_lines)
                .block(Block::default().borders(Borders::ALL).title(" Info "))
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, chunks[2]);
        }
        SettingsSection::ExtensionServer => {
            let mut lines: Vec<Line> = vec![];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(" Extension Server", Style::default().fg(Color::White).bold())));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  URL: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&state.extension_server, Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Packs are fetched from this server.", Style::default().fg(Color::DarkGray))));
            lines.push(Line::from(Span::styled("  Edit settings.toml to change.", Style::default().fg(Color::DarkGray))));
            let content = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Extension Server ")
                    .border_style(Style::default().fg(list_border)));
            // Merge chunks[1] and chunks[2] into one area
            let merged = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width + chunks[2].width, chunks[1].height);
            frame.render_widget(content, merged);
        }
    }
}
