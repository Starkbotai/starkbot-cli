import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, BackendEvent, ChatMessage, ChatSession, SessionSummary, ScheduledTaskSummary, Schedule, FlowDefinition } from "../types";

interface PendingApproval {
  request_id: string;
  tool_name: string;
  args_display: string;
}

export interface DebugLogEntry {
  timestamp: string;
  level: string;
  message: string;
}

interface BackendState {
  messages: ChatMessage[];
  agentBusy: boolean;
  status: string;
  toolActivity: string[];
  personaName: string;
  modelName: string;
  pendingApproval: PendingApproval | null;
  snapshot: AppSnapshot | null;
  debugLogs: DebugLogEntry[];
  viewingSession: ChatSession | null;
  sessions: SessionSummary[];
  scheduledTasks: ScheduledTaskSummary[];
}

export function useBackend() {
  const [state, setState] = useState<BackendState>({
    messages: [],
    agentBusy: false,
    status: "Connecting...",
    toolActivity: [],
    personaName: "",
    modelName: "",
    pendingApproval: null,
    snapshot: null,
    debugLogs: [],
    viewingSession: null,
    sessions: [],
    scheduledTasks: [],
  });

  const messagesRef = useRef(state.messages);
  messagesRef.current = state.messages;

  // Load initial snapshot
  useEffect(() => {
    const applySnapshot = (snapshot: AppSnapshot) => {
      setState((prev) => ({
        ...prev,
        messages: snapshot.messages,
        personaName: snapshot.persona_name,
        modelName: snapshot.model_name,
        status: snapshot.status,
        toolActivity: snapshot.tool_activity,
        agentBusy: snapshot.agent_busy,
        sessions: snapshot.sessions ?? [],
        scheduledTasks: snapshot.scheduled_tasks ?? [],
        snapshot,
      }));
    };
    invoke<AppSnapshot>("get_initial_snapshot")
      .then(applySnapshot)
      .catch((err) => {
        console.error("Failed to get initial snapshot:", err);
        setTimeout(() => {
          invoke<AppSnapshot>("get_initial_snapshot")
            .then(applySnapshot)
            .catch(console.error);
        }, 1000);
      });
  }, []);

  // Listen for backend events
  useEffect(() => {
    const unlisten = listen<BackendEvent>("backend-event", (event) => {
      try {
        // Tauri 2 deserializes the payload for us (Rust Serialize -> JS object)
        const evt: BackendEvent = typeof event.payload === "string"
          ? JSON.parse(event.payload)
          : event.payload;
        applyEvent(evt);
      } catch (e) {
        console.error("[useBackend] event error:", e, event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const applyEvent = useCallback((evt: BackendEvent) => {
    setState((prev) => {
      if ("ToolCall" in evt) {
        const { name, args } = evt.ToolCall;
        const truncArgs = args.length > 80 ? args.slice(0, 80) + "..." : args;
        return {
          ...prev,
          toolActivity: [...prev.toolActivity.slice(-19), `▶ ${name} ${args}`],
          messages: [...prev.messages, { role: "tool", content: `▶ ${name}(${truncArgs})` }],
        };
      }
      if ("ToolResult" in evt) {
        const { name, success, preview } = evt.ToolResult;
        const icon = success ? "✓" : "✗";
        const newActivity = [...prev.toolActivity.slice(-19), `${icon} ${name}`];
        const truncPreview = preview.length > 100 ? preview.slice(0, 100) + "..." : preview;
        const role = success ? "tool" : "error";
        return {
          ...prev,
          toolActivity: newActivity,
          messages: [...prev.messages, { role, content: `${icon} ${name}: ${truncPreview}` }],
        };
      }
      if ("ThinkingText" in evt) {
        const { content } = evt.ThinkingText;
        // Append or update last thinking message
        const lastMsg = prev.messages[prev.messages.length - 1];
        if (lastMsg && lastMsg.role === "thinking") {
          const updatedMessages = [...prev.messages];
          updatedMessages[updatedMessages.length - 1] = { role: "thinking", content: lastMsg.content + content };
          return { ...prev, messages: updatedMessages };
        }
        return {
          ...prev,
          messages: [...prev.messages, { role: "thinking", content }],
        };
      }
      if ("TurnComplete" in evt) {
        return {
          ...prev,
          agentBusy: false,
          status: "Ready",
          messages: [...prev.messages, { role: "assistant", content: evt.TurnComplete.answer }],
        };
      }
      if ("Error" in evt) {
        return {
          ...prev,
          agentBusy: false,
          status: "Ready",
          messages: [...prev.messages, { role: "error", content: evt.Error.message }],
        };
      }
      if ("ApprovalRequired" in evt) {
        return {
          ...prev,
          pendingApproval: {
            request_id: evt.ApprovalRequired.request_id,
            tool_name: evt.ApprovalRequired.tool_name,
            args_display: evt.ApprovalRequired.args_display,
          },
        };
      }
      if ("ModelChanged" in evt) {
        return { ...prev, modelName: evt.ModelChanged.model };
      }
      if ("StatusUpdate" in evt) {
        return { ...prev, agentBusy: evt.StatusUpdate.busy, status: evt.StatusUpdate.message };
      }
      if ("Info" in evt) {
        return {
          ...prev,
          messages: [...prev.messages, { role: "assistant", content: evt.Info.message }],
        };
      }
      if ("DebugLog" in evt) {
        const entry = evt.DebugLog;
        const newLogs = [...prev.debugLogs, entry].slice(-200);
        return { ...prev, debugLogs: newLogs };
      }
      if ("SessionLoaded" in evt) {
        return { ...prev, viewingSession: evt.SessionLoaded };
      }
      if ("SessionsUpdated" in evt) {
        return { ...prev, sessions: evt.SessionsUpdated };
      }
      if ("SchedulesUpdated" in evt) {
        return { ...prev, scheduledTasks: evt.SchedulesUpdated };
      }
      return prev;
    });
  }, []);

  // Actions
  const sendMessage = useCallback(async (content: string) => {
    setState((prev) => ({
      ...prev,
      agentBusy: true,
      status: "Agent thinking...",
      messages: [...prev.messages, { role: "user", content }],
    }));
    await invoke("send_message", { content });
  }, []);

  const approvalResponse = useCallback(async (requestId: string, approved: boolean) => {
    setState((prev) => ({ ...prev, pendingApproval: null }));
    await invoke("approval_response", { requestId, approved });
  }, []);

  const switchModel = useCallback(async (model: string) => {
    await invoke("switch_model", { model });
  }, []);

  const addApiKey = useCallback(async (name: string, key: string) => {
    await invoke("api_key_add", { name, key });
  }, []);

  const deleteApiKey = useCallback(async (name: string) => {
    await invoke("api_key_delete", { name });
  }, []);

  const loadSession = useCallback(async (sessionId: string) => {
    await invoke("load_session", { sessionId });
  }, []);

  const deleteSession = useCallback(async (sessionId: string) => {
    setState((prev) => ({ ...prev, viewingSession: null }));
    await invoke("delete_session", { sessionId });
  }, []);

  const createSchedule = useCallback(async (name: string, schedule: Schedule, flow: FlowDefinition) => {
    await invoke("schedule_create", { name, schedule, flow });
  }, []);

  const deleteSchedule = useCallback(async (taskId: string) => {
    await invoke("schedule_delete", { taskId });
  }, []);

  const toggleSchedule = useCallback(async (taskId: string) => {
    await invoke("schedule_toggle", { taskId });
  }, []);

  return {
    ...state,
    sendMessage,
    approvalResponse,
    switchModel,
    addApiKey,
    deleteApiKey,
    loadSession,
    deleteSession,
    createSchedule,
    deleteSchedule,
    toggleSchedule,
  };
}
