# 07 - graf-rs Integration (Visual Skill Graph Viewer)

## Overview

graf-rs is a terminal-native graph rendering library that provides interactive graph visualization within the ratatui TUI framework. It powers two key views in StarkBot CLI:

1. **Skill Graph** — Visualizes skill relationships and dependencies
2. **Knowledge Graph** — Visualizes memory associations (impulse map equivalent)

## graf-rs Design (Library to Build)

Since graf-rs doesn't exist yet, this document defines what it needs to provide.

### Core API

```rust
pub struct GraphWidget<'a> {
    graph: &'a GraphData,
    layout: LayoutAlgorithm,
    viewport: Viewport,
    style: GraphStyle,
    selection: Option<NodeId>,
}

pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub category: String,      // For coloring
    pub weight: f32,           // For sizing
    pub metadata: Value,       // Arbitrary data
}

pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: String,          // For styling (solid, dashed, etc.)
    pub weight: f32,           // For thickness
}
```

### Layout Algorithms

```rust
pub enum LayoutAlgorithm {
    ForceDirected {
        iterations: usize,
        repulsion: f32,
        attraction: f32,
        damping: f32,
    },
    Hierarchical {
        direction: Direction,  // TopDown, LeftRight
        layer_gap: f32,
        node_gap: f32,
    },
    Radial {
        center_node: Option<String>,
        ring_gap: f32,
    },
    Circular,
}
```

### Rendering in Terminal

```rust
impl<'a> Widget for GraphWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 1. Compute layout positions (cached)
        let positions = self.layout.compute(&self.graph, area);

        // 2. Render edges (Braille characters for smooth lines)
        for edge in &self.graph.edges {
            draw_edge(buf, &positions, edge, &self.style);
        }

        // 3. Render nodes (box-drawing characters)
        for node in &self.graph.nodes {
            draw_node(buf, &positions, node, &self.style,
                      self.selection.as_ref() == Some(&node.id));
        }

        // 4. Render labels
        for node in &self.graph.nodes {
            draw_label(buf, &positions, node);
        }
    }
}
```

### Rendering Techniques

**Edges:** Use Braille Unicode characters (⠁⠂⠄⡀⠈⠐⠠⢀) for sub-character resolution, enabling smooth diagonal lines in the terminal.

**Nodes:** Box-drawing characters for node borders:
```
┌─────────┐
│ planning│
└─────────┘
```

**Highlighted node:**
```
╔═════════╗
║ planning║
╚═════════╝
```

**Edge types:**
- Solid: `─────` (DependsOn)
- Dashed: `- - -` (RelatedTo)
- Arrow: `────▶` (Precedes)
- Double: `═════` (Extends)

### Viewport & Navigation

```rust
pub struct Viewport {
    pub offset_x: f32,
    pub offset_y: f32,
    pub zoom: f32,
}

impl Viewport {
    pub fn pan(&mut self, dx: f32, dy: f32);
    pub fn zoom_in(&mut self);
    pub fn zoom_out(&mut self);
    pub fn fit_to_graph(&mut self, graph: &GraphData, area: Rect);
    pub fn center_on_node(&mut self, node_id: &str, positions: &Positions);
}
```

### Interaction

| Key | Action |
|-----|--------|
| Arrow keys / `hjkl` | Pan viewport |
| `+` / `-` | Zoom in/out |
| `Tab` | Cycle node focus |
| `Enter` | Inspect focused node |
| `f` | Fit graph to viewport |
| `l` | Cycle layout algorithm |
| `/` | Search nodes |
| `Esc` | Deselect / back |

## Skill Graph Integration

```rust
pub fn build_skill_graph(registry: &SkillRegistry) -> GraphData {
    let mut nodes = vec![];
    let mut edges = vec![];

    for skill in registry.all() {
        nodes.push(GraphNode {
            id: skill.name.clone(),
            label: skill.name.clone(),
            category: skill.tags.first().cloned().unwrap_or_default(),
            weight: 1.0,
            metadata: json!({
                "description": skill.description,
                "version": skill.version,
                "tools": skill.requires_tools,
            }),
        });

        // Build edges from requires_tools overlap and tag similarity
        for other in registry.all() {
            if other.name == skill.name { continue; }

            // Shared tools = relationship
            let shared_tools: Vec<_> = skill.requires_tools.iter()
                .filter(|t| other.requires_tools.contains(t))
                .collect();

            if shared_tools.len() >= 2 {
                edges.push(GraphEdge {
                    from: skill.name.clone(),
                    to: other.name.clone(),
                    label: None,
                    kind: "related".into(),
                    weight: shared_tools.len() as f32 / skill.requires_tools.len() as f32,
                });
            }
        }
    }

    GraphData { nodes, edges }
}
```

## Knowledge Graph (Impulse Map) Integration

```rust
pub fn build_memory_graph(associations: &[MemoryAssociation], memories: &[Memory]) -> GraphData {
    let mut nodes = vec![];
    let mut edges = vec![];

    let relevant_ids: HashSet<_> = associations.iter()
        .flat_map(|a| [a.from_id.clone(), a.to_id.clone()])
        .collect();

    for memory in memories.iter().filter(|m| relevant_ids.contains(&m.id)) {
        nodes.push(GraphNode {
            id: memory.id.clone(),
            label: truncate(&memory.content, 20),
            category: format!("{:?}", memory.category),
            weight: memory.importance as f32 / 100.0,
            metadata: json!({
                "content": memory.content,
                "importance": memory.importance,
                "created_at": memory.created_at.to_rfc3339(),
            }),
        });
    }

    for assoc in associations {
        edges.push(GraphEdge {
            from: assoc.from_id.clone(),
            to: assoc.to_id.clone(),
            label: Some(format!("{:?}", assoc.relation)),
            kind: format!("{:?}", assoc.relation),
            weight: assoc.strength,
        });
    }

    GraphData { nodes, edges }
}
```

## Node Inspector Panel

When a node is selected and Enter is pressed:

```
┌─ Node: planning ────────────────────────────────────────┐
│ Category: workflow                                        │
│ Version: 2.1.0                                           │
│ Tags: [workflow, methodology]                            │
│ Tools: [read_file, write_file, define_tasks]             │
│                                                          │
│ Connections:                                             │
│   → debugging (RelatedTo, strength: 0.8)                │
│   → code-review (Precedes, strength: 0.9)               │
│   ← research (DependsOn, strength: 0.7)                 │
│                                                          │
│ [Enter] Load Skill  [Esc] Close                         │
└──────────────────────────────────────────────────────────┘
```

## Performance Considerations

- Layout computation cached (recompute only on graph change or resize)
- Force-directed layout runs async with progressive rendering
- Large graphs (100+ nodes): auto-cluster by category, expand on focus
- Viewport culling: only render visible nodes
- Edge bundling for dense graphs
