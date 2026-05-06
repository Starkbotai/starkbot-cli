import { useState } from "react";
import { useBackend } from "./hooks/useBackend";
import ChatView from "./components/ChatView";
import SkillsView from "./components/SkillsView";
import PersonasView from "./components/PersonasView";
import SettingsView from "./components/SettingsView";
import ApiKeysView from "./components/ApiKeysView";

type View = "chat" | "skills" | "personas" | "settings" | "api-keys";

const TABS: { id: View; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "skills", label: "Skills" },
  { id: "personas", label: "Personas" },
  { id: "api-keys", label: "API Keys" },
  { id: "settings", label: "Settings" },
];

export default function App() {
  const [activeView, setActiveView] = useState<View>("chat");
  const backend = useBackend();

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
      </div>
    </div>
  );
}
