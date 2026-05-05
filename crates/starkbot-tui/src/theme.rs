use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub user_msg: Color,
    pub agent_msg: Color,
    pub tool_msg: Color,
    pub error_msg: Color,
    pub dim: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            accent: Color::Cyan,
            user_msg: Color::Green,
            agent_msg: Color::Cyan,
            tool_msg: Color::Yellow,
            error_msg: Color::Red,
            dim: Color::DarkGray,
        }
    }
}
