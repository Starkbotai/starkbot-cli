import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, BackendEvent, ChatMessage, ChatSession, SessionSummary, SavedFlow, FlowSummary, FlowLogEntry, FlowTemplateInfo, CustomFileEntry, InternalEvent, PackInfo, ChannelInfo, ChannelSettingInfo } from "../types";

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
  flows: FlowSummary[];
  flowLogs: FlowLogEntry[];
  editingFlow: SavedFlow | null;
  inferenceConfigured: boolean;
  runningFlows: number;
  flowTemplates: FlowTemplateInfo[];
  internalEvents: InternalEvent[];
  remotePacks: PackInfo[];
  packsLoading: boolean;
  packsMessage: string | null;
  channels: ChannelInfo[];
  channelSettings: ChannelSettingInfo[];
  selectedChannelId: string | null;
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
    flows: [],
    flowLogs: [],
    editingFlow: null,
    inferenceConfigured: false,
    runningFlows: 0,
    flowTemplates: [],
    internalEvents: [],
    remotePacks: [],
    packsLoading: false,
    packsMessage: null,
    channels: [],
    channelSettings: [],
    selectedChannelId: null,
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
        inferenceConfigured: snapshot.inference_configured ?? false,
        channels: snapshot.channels ?? [],
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
      if ("FlowLoaded" in evt) {
        return { ...prev, editingFlow: evt.FlowLoaded };
      }
      if ("FlowsListed" in evt) {
        return { ...prev, flows: evt.FlowsListed };
      }
      if ("FlowLogsLoaded" in evt) {
        return { ...prev, flowLogs: evt.FlowLogsLoaded };
      }
      if ("FlowRunningCount" in evt) {
        return { ...prev, runningFlows: evt.FlowRunningCount.count };
      }
      if ("FlowTemplatesListed" in evt) {
        return { ...prev, flowTemplates: evt.FlowTemplatesListed };
      }
      if ("EventsLogUpdated" in evt) {
        return { ...prev, internalEvents: evt.EventsLogUpdated };
      }
      if ("IntegrationsUpdated" in evt) {
        const updatedSnapshot = prev.snapshot
          ? { ...prev.snapshot, integrations: evt.IntegrationsUpdated }
          : prev.snapshot;
        return { ...prev, snapshot: updatedSnapshot };
      }
      if ("PacksListed" in evt) {
        return { ...prev, remotePacks: evt.PacksListed, packsLoading: false, packsMessage: null };
      }
      if ("PackInstalled" in evt) {
        const updated = prev.remotePacks.map((p) =>
          p.slug === evt.PackInstalled.slug ? { ...p, installed: true } : p
        );
        return { ...prev, remotePacks: updated, packsLoading: false, packsMessage: `Installed '${evt.PackInstalled.slug}'` };
      }
      if ("PackError" in evt) {
        return { ...prev, packsLoading: false, packsMessage: `Error: ${evt.PackError.message}` };
      }
      if ("ChannelsUpdated" in evt) {
        return { ...prev, channels: evt.ChannelsUpdated };
      }
      if ("ChannelSettingsLoaded" in evt) {
        return {
          ...prev,
          channelSettings: evt.ChannelSettingsLoaded.settings,
          selectedChannelId: evt.ChannelSettingsLoaded.channel_id,
        };
      }
      if ("GatewayMessage" in evt) {
        const { channel_name, user_name, text } = evt.GatewayMessage;
        const truncText = text.length > 100 ? text.slice(0, 100) + "..." : text;
        return {
          ...prev,
          messages: [...prev.messages, { role: "tool", content: `[gateway:${channel_name}] ${user_name}: ${truncText}` }],
        };
      }
      if ("GatewayResponse" in evt) {
        // Response logged via agent turn
        return prev;
      }
      if ("Snapshot" in evt) {
        const snapshot = evt.Snapshot;
        return {
          ...prev,
          messages: snapshot.messages,
          personaName: snapshot.persona_name,
          modelName: snapshot.model_name,
          status: snapshot.status,
          toolActivity: snapshot.tool_activity,
          agentBusy: snapshot.agent_busy,
          sessions: snapshot.sessions ?? [],
          inferenceConfigured: snapshot.inference_configured ?? false,
          channels: snapshot.channels ?? [],
          snapshot,
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
    const masked_key = key.length > 8
      ? key.slice(0, 3) + "..." + key.slice(-4)
      : "***";
    setState((prev) => {
      const existingKeys = prev.snapshot?.api_keys ?? [];
      const updatedKeys = existingKeys.filter((k) => k.name !== name);
      updatedKeys.push({ name, masked_key });
      const updatedSnapshot = prev.snapshot
        ? { ...prev.snapshot, api_keys: updatedKeys, inference_configured: name === "OPENAI_API_KEY" ? true : prev.snapshot.inference_configured }
        : prev.snapshot;
      return {
        ...prev,
        inferenceConfigured: name === "OPENAI_API_KEY" ? true : prev.inferenceConfigured,
        snapshot: updatedSnapshot,
      };
    });
  }, []);

  const deleteApiKey = useCallback(async (name: string) => {
    await invoke("api_key_delete", { name });
    if (name === "OPENAI_API_KEY") {
      setState((prev) => ({ ...prev, inferenceConfigured: false }));
    }
  }, []);

  const loadSession = useCallback(async (sessionId: string) => {
    await invoke("load_session", { sessionId });
  }, []);

  const deleteSession = useCallback(async (sessionId: string) => {
    setState((prev) => ({ ...prev, viewingSession: null }));
    await invoke("delete_session", { sessionId });
  }, []);

  const saveFlow = useCallback(async (flow: SavedFlow) => {
    await invoke("flow_save", { flow });
  }, []);

  const loadFlow = useCallback(async (flowId: string) => {
    await invoke("flow_load", { flowId });
  }, []);

  const deleteFlow = useCallback(async (flowId: string) => {
    await invoke("flow_delete", { flowId });
  }, []);

  const listFlows = useCallback(async () => {
    await invoke("flow_list");
  }, []);

  const toggleFlowEnabled = useCallback(async (flowId: string) => {
    await invoke("flow_toggle_enabled", { flowId });
  }, []);

  const runFlowOnce = useCallback(async (flowId: string) => {
    await invoke("flow_run_once", { flowId });
  }, []);

  const loadFlowLogs = useCallback(async () => {
    await invoke("flow_logs_load");
  }, []);

  const loadEventsLog = useCallback(async () => {
    await invoke("events_log_load");
  }, []);

  const slashCommand = useCallback(async (command: string) => {
    // Optimistically clear local state for /clear and /new
    if (command === "/clear" || command === "/new") {
      setState((prev) => ({ ...prev, messages: [], toolActivity: [] }));
    }
    await invoke("slash_command", { command });
  }, []);

  const clearMessages = useCallback(() => {
    setState((prev) => ({ ...prev, messages: [], toolActivity: [] }));
  }, []);

  const clearEditingFlow = useCallback(() => {
    setState((prev) => ({ ...prev, editingFlow: null }));
  }, []);

  const installIntegration = useCallback(async (presetId: string, apiKeys: [string, string][]) => {
    await invoke("integration_install", { presetId, apiKeys });
  }, []);

  const uninstallIntegration = useCallback(async (presetId: string) => {
    await invoke("integration_uninstall", { presetId });
  }, []);

  const importIntegrationFlow = useCallback(async (presetId: string) => {
    await invoke("integration_import_flow", { presetId });
  }, []);

  const listFlowTemplates = useCallback(async () => {
    await invoke("flow_list_templates");
  }, []);

  const listCustomFiles = useCallback(async (): Promise<CustomFileEntry[]> => {
    return await invoke("list_custom_files");
  }, []);

  const readCustomFile = useCallback(async (path: string): Promise<string> => {
    return await invoke("read_custom_file", { path });
  }, []);

  const writeCustomFile = useCallback(async (path: string, content: string): Promise<void> => {
    await invoke("write_custom_file", { path, content });
  }, []);

  const listPacks = useCallback(async () => {
    setState((prev) => ({ ...prev, packsLoading: true, packsMessage: "Fetching packs..." }));
    await invoke("packs_list");
  }, []);

  const installPack = useCallback(async (slug: string) => {
    setState((prev) => ({ ...prev, packsLoading: true, packsMessage: `Installing '${slug}'...` }));
    await invoke("pack_install", { slug });
  }, []);

  const createChannel = useCallback(async (channelType: string, name: string) => {
    await invoke("channel_create", { channelType, name });
  }, []);

  const deleteChannel = useCallback(async (channelId: string) => {
    setState((prev) => ({ ...prev, channelSettings: [], selectedChannelId: null }));
    await invoke("channel_delete", { channelId });
  }, []);

  const startChannel = useCallback(async (channelId: string) => {
    await invoke("channel_start", { channelId });
  }, []);

  const stopChannel = useCallback(async (channelId: string) => {
    await invoke("channel_stop", { channelId });
  }, []);

  const updateChannelSetting = useCallback(async (channelId: string, key: string, value: string) => {
    await invoke("channel_setting_update", { channelId, key, value });
  }, []);

  const loadChannelSettings = useCallback(async (channelId: string) => {
    await invoke("channel_settings_load", { channelId });
  }, []);

  const listChannels = useCallback(async () => {
    await invoke("channels_list");
  }, []);

  const uninstallPack = useCallback(async (slug: string) => {
    setState((prev) => ({
      ...prev,
      remotePacks: prev.remotePacks.map((p) => p.slug === slug ? { ...p, installed: false } : p),
      packsMessage: `Uninstalled '${slug}'`,
    }));
    await invoke("pack_uninstall", { slug });
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
    saveFlow,
    loadFlow,
    deleteFlow,
    listFlows,
    toggleFlowEnabled,
    runFlowOnce,
    loadFlowLogs,
    loadEventsLog,
    slashCommand,
    clearMessages,
    clearEditingFlow,
    installIntegration,
    uninstallIntegration,
    importIntegrationFlow,
    listFlowTemplates,
    listCustomFiles,
    readCustomFile,
    writeCustomFile,
    listPacks,
    installPack,
    uninstallPack,
    createChannel,
    deleteChannel,
    startChannel,
    stopChannel,
    updateChannelSetting,
    loadChannelSettings,
    listChannels,
  };
}
