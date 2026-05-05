pub mod views;
pub mod widgets;
pub mod theme;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};

use starkbot_graph::{GraphData, GraphWidget, Viewport};

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveView {
    Chat,
    Skills,
    Graph,
    Memory,
}

impl ActiveView {
    pub fn titles() -> Vec<&'static str> {
        vec!["Chat", "Skills", "Graph", "Memory"]
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Chat => 0,
            Self::Skills => 1,
            Self::Graph => 2,
            Self::Memory => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Skills,
            2 => Self::Graph,
            3 => Self::Memory,
            _ => Self::Chat,
        }
    }

    pub fn next(&self) -> Self {
        Self::from_index((self.index() + 1) % 4)
    }
}

/// A chat message in the TUI.
#[derive(Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
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
    pub selected_skill: usize,
    pub should_quit: bool,
    pub agent_busy: bool,
}

impl TuiState {
    pub fn new(persona_name: &str, model_name: &str) -> Self {
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
            selected_skill: 0,
            should_quit: false,
            agent_busy: false,
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

/// Handle a key event, returns Some(input) if user submitted a message.
pub fn handle_key(state: &mut TuiState, key: KeyEvent) -> Option<String> {
    // Global keys
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.should_quit = true;
        return None;
    }

    match state.active_view {
        ActiveView::Chat => handle_chat_key(state, key),
        ActiveView::Skills => { handle_skills_key(state, key); None }
        ActiveView::Graph => { handle_graph_key(state, key); None }
        ActiveView::Memory => { handle_memory_key(state, key); None }
    }
}

fn handle_chat_key(state: &mut TuiState, key: KeyEvent) -> Option<String> {
    match key.code {
        KeyCode::Tab => { state.active_view = state.active_view.next(); None }
        KeyCode::Enter => {
            if state.agent_busy || state.input.is_empty() { return None; }
            let input = state.input.clone();
            state.input.clear();
            state.input_cursor = 0;
            Some(input)
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

/// Draw the full TUI frame.
pub fn draw(frame: &mut ratatui::Frame, state: &TuiState) {
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
        ActiveView::Memory => draw_memory(frame, state, chunks[1]),
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
        Span::styled("Tab: switch │ Ctrl+C: quit", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[2]);
}

fn draw_chat(frame: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),     // Messages
            Constraint::Length(3),  // Input
            Constraint::Length(3),  // Tool activity
        ])
        .split(area);

    // Messages
    let mut lines: Vec<Line> = vec![];
    for msg in &state.messages {
        let (prefix, color) = match msg.role.as_str() {
            "user" => ("[you]", Color::Green),
            "assistant" => ("[agent]", Color::Cyan),
            "tool" => ("[tool]", Color::Yellow),
            "error" => ("[error]", Color::Red),
            _ => ("[?]", Color::Gray),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", prefix), Style::default().fg(color).bold()),
            Span::raw(&msg.content),
        ]));
    }
    let messages = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Messages "))
        .wrap(Wrap { trim: false })
        .scroll((state.messages.len().saturating_sub(chunks[0].height as usize) as u16, 0));
    frame.render_widget(messages, chunks[0]);

    // Input
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

    // Skill list
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
        .block(Block::default().borders(Borders::ALL).title(" Skills "));
    frame.render_widget(list, chunks[0]);

    // Skill detail (placeholder)
    let detail_text = if state.skill_names.is_empty() {
        "No skills loaded".to_string()
    } else if state.selected_skill < state.skill_names.len() {
        format!("Selected: {}\n\nUse ↑↓ to navigate", state.skill_names[state.selected_skill])
    } else {
        String::new()
    };
    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(" Detail "))
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
