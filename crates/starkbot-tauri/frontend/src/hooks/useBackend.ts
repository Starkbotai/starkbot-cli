import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, BackendEvent, ChatMessage } from "../types";

interface PendingApproval {
  request_id: string;
  tool_name: string;
  args_display: string;
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
  });

  const messagesRef = useRef(state.messages);
  messagesRef.current = state.messages;

  // Load initial snapshot
  useEffect(() => {
    invoke<AppSnapshot>("get_initial_snapshot")
      .then((snapshot) => {
        setState((prev) => ({
          ...prev,
          messages: snapshot.messages,
          personaName: snapshot.persona_name,
          modelName: snapshot.model_name,
          status: snapshot.status,
          toolActivity: snapshot.tool_activity,
          agentBusy: snapshot.agent_busy,
          snapshot,
        }));
      })
      .catch((err) => {
        console.error("Failed to get initial snapshot:", err);
        // Retry after a short delay (engine may still be starting)
        setTimeout(() => {
          invoke<AppSnapshot>("get_initial_snapshot")
            .then((snapshot) => {
              setState((prev) => ({
                ...prev,
                messages: snapshot.messages,
                personaName: snapshot.persona_name,
                modelName: snapshot.model_name,
                status: snapshot.status,
                toolActivity: snapshot.tool_activity,
                agentBusy: snapshot.agent_busy,
                snapshot,
              }));
            })
            .catch(console.error);
        }, 1000);
      });
  }, []);

  // Listen for backend events
  useEffect(() => {
    const unlisten = listen<string>("backend-event", (event) => {
      const evt: BackendEvent = JSON.parse(event.payload);
      applyEvent(evt);
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
        if (!success) {
          const truncPreview = preview.length > 100 ? preview.slice(0, 100) + "..." : preview;
          return {
            ...prev,
            toolActivity: newActivity,
            messages: [...prev.messages, { role: "error", content: `${icon} ${name} failed: ${truncPreview}` }],
          };
        }
        return { ...prev, toolActivity: newActivity };
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

  return {
    ...state,
    sendMessage,
    approvalResponse,
    switchModel,
    addApiKey,
    deleteApiKey,
  };
}
