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
  node_type: "entry" | "prompt" | "branch" | "branch_tool";
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

export interface SavedFlow {
  id: string;
  name: string;
  flow: FlowDefinition;
  created_at: string;
  updated_at: string;
  enabled: boolean;
}

export interface FlowSummary {
  id: string;
  name: string;
  node_count: number;
  created_at: string;
  updated_at: string;
  enabled: boolean;
}

export interface FlowLogEntry {
  timestamp: string;
  flow_id: string;
  flow_name: string;
  action: string;
  detail: string;
  run_id?: string;
}

export interface RequiredKeyInfo {
  name: string;
  label: string;
}

export interface IntegrationPresetInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
  api_key_name: string | null;
  required_keys: RequiredKeyInfo[];
  skills: string[];
  installed: boolean;
  configured: boolean;
  has_flow_template: boolean;
}

export interface FlowTemplateInfo {
  preset_id: string;
  preset_name: string;
  template_name: string;
}

export interface CustomFileEntry {
  path: string;
  name: string;
  is_dir: boolean;
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
  sessions_dir: string;
  flows_dir: string;
  inference_configured: boolean;
  integrations: IntegrationPresetInfo[];
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
  | { FlowLoaded: SavedFlow }
  | { FlowsListed: FlowSummary[] }
  | { FlowLogsLoaded: FlowLogEntry[] }
  | { FlowRunningCount: { count: number } }
  | { FlowTemplatesListed: FlowTemplateInfo[] }
  | { IntegrationsUpdated: IntegrationPresetInfo[] };
