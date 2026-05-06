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
}

// BackendEvent variants (comes as JSON string from Tauri)
export type BackendEvent =
  | { ToolCall: { name: string; args: string } }
  | { ToolResult: { name: string; success: boolean; preview: string } }
  | { TurnComplete: { answer: string } }
  | { Error: { message: string } }
  | { ApprovalRequired: { request_id: string; tool_name: string; args_display: string } }
  | { ModelChanged: { model: string } }
  | { StatusUpdate: { busy: boolean; message: string } }
  | { Info: { message: string } }
  | { Snapshot: AppSnapshot }
  | { DebugLog: { timestamp: string; level: string; message: string } };
