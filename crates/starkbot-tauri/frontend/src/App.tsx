import { useState, useRef, useEffect } from "react";
import { useBackend } from "./hooks/useBackend";
import ChatView from "./components/ChatView";
import SkillsView from "./components/SkillsView";
import PersonasView from "./components/PersonasView";
import SettingsView from "./components/SettingsView";
import ApiKeysView from "./components/ApiKeysView";
import DataView from "./components/DataView";
import SchedulingView from "./components/SchedulingView";

type View = "chat" | "skills" | "personas" | "data" | "scheduling" | "api-keys" | "settings";

const TABS: { id: View; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "skills", label: "Skills" },
  { id: "personas", label: "Personas" },
  { id: "data", label: "Data" },
  { id: "scheduling", label: "Scheduling" },
  { id: "api-keys", label: "API Keys" },
  { id: "settings", label: "Settings" },
];

export default function App() {
  const [activeView, setActiveView] = useState<View>("chat");
  const [showDebug, setShowDebug] = useState(false);
  const debugEndRef = useRef<HTMLDivElement>(null);
  const backend = useBackend();

  useEffect(() => {
    if (showDebug && debugEndRef.current) {
      debugEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [backend.debugLogs, showDebug]);

  return (
    <div className="h-screen flex flex-col bg-surface-0">
      {/* Tab bar */}
      <div className="flex items-center gap-1 px-3 py-1.5 bg-surface-1 border-b border-surface-3">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveView(tab.id)}
            className={`px-3 py-1 text-sm rounded transition-colors ${
              activeView === tab.id
                ? "bg-accent text-white"
                : "text-gray-400 hover:text-gray-200 hover:bg-surface-2"
            }`}
          >
            {tab.label}
          </button>
        ))}
        <div className="flex-1" />
        <span className="text-xs text-gray-500">
          {backend.personaName} | {backend.modelName}
        </span>
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-hidden">
        {activeView === "chat" && <ChatView backend={backend} />}
        {activeView === "skills" && <SkillsView snapshot={backend.snapshot} />}
        {activeView === "personas" && <PersonasView snapshot={backend.snapshot} />}
        {activeView === "data" && (
          <DataView
            sessions={backend.sessions}
            viewingSession={backend.viewingSession}
            onLoadSession={backend.loadSession}
            onDeleteSession={backend.deleteSession}
          />
        )}
        {activeView === "scheduling" && (
          <SchedulingView
            tasks={backend.scheduledTasks}
            onCreate={backend.createSchedule}
            onDelete={backend.deleteSchedule}
            onToggle={backend.toggleSchedule}
          />
        )}
        {activeView === "api-keys" && (
          <ApiKeysView
            snapshot={backend.snapshot}
            onAdd={backend.addApiKey}
            onDelete={backend.deleteApiKey}
          />
        )}
        {activeView === "settings" && (
          <SettingsView
            snapshot={backend.snapshot}
            currentModel={backend.modelName}
            onSwitchModel={backend.switchModel}
          />
        )}
      </div>

      {/* Debug panel */}
      {showDebug && (
        <div className="h-48 border-t border-surface-3 bg-black/80 overflow-y-auto font-mono text-[11px] text-gray-300 px-2 py-1">
          {backend.debugLogs.map((log, i) => (
            <div key={i} className="whitespace-nowrap">
              <span className="text-gray-500">{log.timestamp}</span>{" "}
              <span className={log.level === "ERROR" ? "text-red-400" : log.level === "WARN" ? "text-yellow-400" : "text-blue-400"}>
                {log.level}
              </span>{" "}
              <span>{log.message}</span>
            </div>
          ))}
          <div ref={debugEndRef} />
        </div>
      )}

      {/* Status bar */}
      <div className="flex items-center gap-3 px-3 py-1 bg-surface-1 border-t border-surface-3 text-xs">
        <span className={backend.agentBusy ? "text-yellow-400" : "text-green-400"}>
          {backend.agentBusy ? "⟳ Agent thinking..." : "Ready"}
        </span>
        <span className="text-gray-600">|</span>
        <span className="text-gray-500">{backend.status}</span>
        {backend.toolActivity.length > 0 && (
          <>
            <span className="text-gray-600">|</span>
            <span className="text-gray-500 truncate">
              {backend.toolActivity[backend.toolActivity.length - 1]}
            </span>
          </>
        )}
        <div className="flex-1" />
        <button
          onClick={() => setShowDebug(!showDebug)}
          className={`px-2 py-0.5 rounded text-[10px] ${
            showDebug ? "bg-accent text-white" : "text-gray-500 hover:text-gray-300 hover:bg-surface-2"
          }`}
        >
          Debug {backend.debugLogs.length > 0 && `(${backend.debugLogs.length})`}
        </button>
      </div>
    </div>
  );
}
