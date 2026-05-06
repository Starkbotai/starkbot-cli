// Mirrors starkbot-api types for the frontend

export interface ChatMessage {
  role: string;
  content: string;
}

export interface PersonaInfo {
  key: string;
  label: string;
  description: string;
  emoji: string;
  enabled: boolean;
  tool_groups: string[];
  skill_tags: string[];
  system_prompt_preview: string;
}

export interface SkillInfo {
  name: string;
  description: string;
  version: string;
  tags: string[];
  requires_tools: string[];
  content: string;
}

export interface ApiKeyInfo {
  name: string;
  masked_key: string;
}

export interface GraphNodeDto {
  id: string;
  label: string;
  category: string;
  weight: number;
}

export interface GraphEdgeDto {
  from: string;
  to: string;
  label: string | null;
  kind: string;
  weight: number;
}

export interface ChatSessionMessage {
  role: string;
  content: string;
}

export interface ChatSession {
  id: string;
  persona: string;
  title: string;
  created_at: string;
  messages: ChatSessionMessage[];
}

export interface SessionSummary {
  id: string;
  persona: string;
  title: string;
  created_at: string;
  message_count: number;
}

export interface FlowNode {
  id: string;
  node_type: "prompt" | "branch";
  data: Record<string, unknown>;
  position: [number, number];
}

export interface FlowEdge {
  id: string;
  source: string;
  target: string;
  source_handle?: string;
  target_handle?: string;
}

export interface FlowDefinition {
  nodes: FlowNode[];
  edges: FlowEdge[];
}

export type Schedule =
  | { type: "every_minutes"; value: number }
  | { type: "every_hours"; value: number };

export interface ScheduledTask {
  id: string;
  name: string;
  schedule: Schedule;
  flow: FlowDefinition;
  created_at: string;
  enabled: boolean;
}

export interface ScheduledTaskSummary {
  id: string;
  name: string;
  schedule: Schedule;
  node_count: number;
  enabled: boolean;
  created_at: string;
}

export interface AppSnapshot {
  persona_name: string;
  model_name: string;
  agent_busy: boolean;
  status: string;
  messages: ChatMessage[];
  tool_activity: string[];
  skills: SkillInfo[];
  personas: PersonaInfo[];
  api_keys: ApiKeyInfo[];
  available_models: string[];
  graph_nodes: GraphNodeDto[];
  graph_edges: GraphEdgeDto[];
  skills_dir: string;
  agents_dir: string;
  sessions: SessionSummary[];
  scheduled_tasks: ScheduledTaskSummary[];
  sessions_dir: string;
  schedules_dir: string;
}

// BackendEvent variants (comes as JSON string from Tauri)
export type BackendEvent =
  | { ToolCall: { name: string; args: string } }
  | { ToolResult: { name: string; success: boolean; preview: string } }
  | { ThinkingText: { content: string } }
  | { TurnComplete: { answer: string } }
  | { Error: { message: string } }
  | { ApprovalRequired: { request_id: string; tool_name: string; args_display: string } }
  | { ModelChanged: { model: string } }
  | { StatusUpdate: { busy: boolean; message: string } }
  | { Info: { message: string } }
  | { Snapshot: AppSnapshot }
  | { DebugLog: { timestamp: string; level: string; message: string } }
  | { SessionLoaded: ChatSession }
  | { SessionsUpdated: SessionSummary[] }
  | { SchedulesUpdated: ScheduledTaskSummary[] };
