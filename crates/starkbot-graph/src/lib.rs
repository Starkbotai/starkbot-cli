#[cfg(feature = "tui")]
use ratatui::buffer::Buffer;
#[cfg(feature = "tui")]
use ratatui::layout::Rect;
#[cfg(feature = "tui")]
use ratatui::style::{Color, Style};
#[cfg(feature = "tui")]
use ratatui::widgets::Widget;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub category: String,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Default)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone)]
struct Position {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom: f64,
    pub selected: Option<usize>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { offset_x: 0.0, offset_y: 0.0, zoom: 1.0, selected: None }
    }
}

impl Viewport {
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.2);
    }

    pub fn select_next(&mut self, node_count: usize) {
        if node_count == 0 { return; }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % node_count,
            None => 0,
        });
    }

    pub fn select_prev(&mut self, node_count: usize) {
        if node_count == 0 { return; }
        self.selected = Some(match self.selected {
            Some(0) | None => node_count.saturating_sub(1),
            Some(i) => i - 1,
        });
    }
}

/// Build a graph from skill data for visualization.
pub fn build_skill_graph(skills: &[(String, String, Vec<String>)]) -> GraphData {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for (name, _desc, tags) in skills {
        nodes.push(GraphNode {
            id: name.clone(),
            label: name.clone(),
            category: tags.first().cloned().unwrap_or_else(|| "default".into()),
            weight: 1.0,
        });
    }

    // Build edges from shared tags
    for i in 0..skills.len() {
        for j in (i + 1)..skills.len() {
            let shared: usize = skills[i].2.iter()
                .filter(|t| skills[j].2.contains(t))
                .count();
            if shared > 0 {
                edges.push(GraphEdge {
                    from: skills[i].0.clone(),
                    to: skills[j].0.clone(),
                    label: None,
                    kind: "related".into(),
                    weight: shared as f32,
                });
            }
        }
    }

    GraphData { nodes, edges }
}

// --- TUI Widget (only available with `tui` feature) ---

#[cfg(feature = "tui")]
/// Compute force-directed layout positions.
fn layout_force_directed(graph: &GraphData, width: f64, height: f64) -> Vec<Position> {
    let n = graph.nodes.len();
    if n == 0 { return vec![]; }

    let mut positions: Vec<Position> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            Position {
                x: width / 2.0 + (width * 0.35) * angle.cos(),
                y: height / 2.0 + (height * 0.35) * angle.sin(),
            }
        })
        .collect();

    let node_index = |id: &str| -> Option<usize> {
        graph.nodes.iter().position(|n| n.id == id)
    };

    let ideal_dist = ((width * height) / n as f64).sqrt().max(4.0);
    let iterations = 50;

    for iter in 0..iterations {
        let temperature = 1.0 - iter as f64 / iterations as f64;
        let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dx = positions[i].x - positions[j].x;
                let dy = positions[i].y - positions[j].y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                let force = (ideal_dist * ideal_dist) / dist;
                let fx = dx / dist * force * temperature;
                let fy = dy / dist * force * temperature;
                forces[i].0 += fx;
                forces[i].1 += fy;
                forces[j].0 -= fx;
                forces[j].1 -= fy;
            }
        }

        for edge in &graph.edges {
            if let (Some(i), Some(j)) = (node_index(&edge.from), node_index(&edge.to)) {
                let dx = positions[i].x - positions[j].x;
                let dy = positions[i].y - positions[j].y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                let force = (dist * dist) / ideal_dist;
                let fx = dx / dist * force * temperature * 0.3;
                let fy = dy / dist * force * temperature * 0.3;
                forces[i].0 -= fx;
                forces[i].1 -= fy;
                forces[j].0 += fx;
                forces[j].1 += fy;
            }
        }

        let max_move = ideal_dist * temperature;
        for i in 0..n {
            let mag = (forces[i].0.powi(2) + forces[i].1.powi(2)).sqrt().max(0.001);
            let capped = mag.min(max_move);
            positions[i].x += forces[i].0 / mag * capped;
            positions[i].y += forces[i].1 / mag * capped;
            positions[i].x = positions[i].x.clamp(2.0, width - 2.0);
            positions[i].y = positions[i].y.clamp(1.0, height - 1.0);
        }
    }

    positions
}

#[cfg(feature = "tui")]
fn category_color(category: &str) -> Color {
    match category {
        "workflow" | "methodology" => Color::Cyan,
        "development" | "code" => Color::Green,
        "operations" | "devops" => Color::Yellow,
        "research" => Color::Blue,
        "creative" => Color::Magenta,
        "fact" => Color::White,
        "preference" => Color::LightYellow,
        "entity" => Color::LightGreen,
        "identity" => Color::LightRed,
        _ => Color::Gray,
    }
}

#[cfg(feature = "tui")]
pub struct GraphWidget<'a> {
    graph: &'a GraphData,
    viewport: &'a Viewport,
}

#[cfg(feature = "tui")]
impl<'a> GraphWidget<'a> {
    pub fn new(graph: &'a GraphData, viewport: &'a Viewport) -> Self {
        Self { graph, viewport }
    }
}

#[cfg(feature = "tui")]
impl Widget for GraphWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.graph.nodes.is_empty() {
            let msg = "No nodes to display";
            let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
            let y = area.y + area.height / 2;
            if y < area.y + area.height && x < area.x + area.width {
                buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            }
            return;
        }

        let w = area.width as f64;
        let h = area.height as f64;
        let positions = layout_force_directed(self.graph, w, h);

        let node_index = |id: &str| -> Option<usize> {
            self.graph.nodes.iter().position(|n| n.id == id)
        };

        let to_screen = |pos: &Position| -> (u16, u16) {
            let sx = ((pos.x + self.viewport.offset_x) * self.viewport.zoom) as i32;
            let sy = ((pos.y + self.viewport.offset_y) * self.viewport.zoom) as i32;
            (
                (area.x as i32 + sx).clamp(area.x as i32, (area.x + area.width - 1) as i32) as u16,
                (area.y as i32 + sy).clamp(area.y as i32, (area.y + area.height - 1) as i32) as u16,
            )
        };

        // Draw edges
        for edge in &self.graph.edges {
            if let (Some(fi), Some(ti)) = (node_index(&edge.from), node_index(&edge.to)) {
                let (x1, y1) = to_screen(&positions[fi]);
                let (x2, y2) = to_screen(&positions[ti]);
                draw_line(buf, area, x1, y1, x2, y2, Style::default().fg(Color::DarkGray));
            }
        }

        // Draw nodes
        for (i, node) in self.graph.nodes.iter().enumerate() {
            let (sx, sy) = to_screen(&positions[i]);
            if sx >= area.x && sx < area.x + area.width && sy >= area.y && sy < area.y + area.height {
                let is_selected = self.viewport.selected == Some(i);
                let color = category_color(&node.category);
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(color)
                } else {
                    Style::default().fg(color)
                };
                let marker = if is_selected { "●" } else { "◉" };
                buf.set_string(sx, sy, marker, style);

                let label = if node.label.len() > 12 {
                    format!("{}...", &node.label[..9])
                } else {
                    node.label.clone()
                };
                let lx = sx.saturating_add(2);
                if lx + label.len() as u16 <= area.x + area.width && sy < area.y + area.height {
                    buf.set_string(lx, sy, &label, Style::default().fg(color));
                }
            }
        }

        // Status line
        let status = format!(
            " Nodes: {} | Edges: {} | Zoom: {:.0}% ",
            self.graph.nodes.len(),
            self.graph.edges.len(),
            self.viewport.zoom * 100.0,
        );
        let sx = area.x + area.width.saturating_sub(status.len() as u16 + 1);
        let sy = area.y + area.height.saturating_sub(1);
        if sy < area.y + area.height {
            buf.set_string(sx, sy, &status, Style::default().fg(Color::DarkGray));
        }
    }
}

#[cfg(feature = "tui")]
/// Draw a simple line using Bresenham's algorithm.
fn draw_line(buf: &mut Buffer, area: Rect, x1: u16, y1: u16, x2: u16, y2: u16, style: Style) {
    let (mut x, mut y) = (x1 as i32, y1 as i32);
    let (dx, dy) = ((x2 as i32 - x1 as i32).abs(), -(y2 as i32 - y1 as i32).abs());
    let (sx, sy) = (
        if x1 < x2 { 1 } else { -1 },
        if y1 < y2 { 1 } else { -1 },
    );
    let mut err = dx + dy;

    let chars = ['·', '─', '│', '╲', '╱'];
    loop {
        let ux = x as u16;
        let uy = y as u16;
        if ux >= area.x && ux < area.x + area.width && uy >= area.y && uy < area.y + area.height {
            let ch = if dx.abs() > dy.abs() * 2 { chars[1] }
                else if dy.abs() > dx.abs() * 2 { chars[2] }
                else { chars[0] };
            buf.set_string(ux, uy, &ch.to_string(), style);
        }
        if x == x2 as i32 && y == y2 as i32 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}
