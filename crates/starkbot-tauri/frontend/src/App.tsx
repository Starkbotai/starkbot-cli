import { useState, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useBackend } from "./hooks/useBackend";
import ChatView from "./components/ChatView";
import SkillsView from "./components/SkillsView";
import PersonasView from "./components/PersonasView";
import SettingsView from "./components/SettingsView";
import ApiKeysView from "./components/ApiKeysView";
import DataView from "./components/DataView";
import SchedulingView from "./components/SchedulingView";
import PacksView from "./components/PacksView";
import GatewayView from "./components/GatewayView";
import SkillTestsView from "./components/SkillTestsView";

type View = "chat" | "skills" | "personas" | "data" | "flows" | "api-keys" | "packs" | "gateway" | "skill-tests" | "settings";

const TABS: { id: View; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "skills", label: "Skills" },
  { id: "personas", label: "Personas" },
  { id: "data", label: "Data" },
  { id: "flows", label: "Flows" },
  { id: "api-keys", label: "Integrations" },
  { id: "gateway", label: "Gateway" },
  { id: "skill-tests", label: "Skill Tests" },
  { id: "settings", label: "Settings" },
];

function OpenFolderButton({ view, snapshot }: { view: View; snapshot: any }) {
  const dir = useMemo(() => {
    if (!snapshot) return "";
    switch (view) {
      case "skills": return snapshot.skills_dir || "";
      case "personas": return snapshot.agents_dir || "";
      case "data": return snapshot.sessions_dir || "";
      case "flows": return snapshot.flows_dir || "";
      case "api-keys": return snapshot.skills_dir ? snapshot.skills_dir.replace(/\/skills$/, "") : "";
      case "settings": return snapshot.skills_dir ? snapshot.skills_dir.replace(/\/skills$/, "") : "";
      case "skill-tests": return snapshot.skill_tests_dir || "";
      default: return "";
    }
  }, [view, snapshot]);

  if (!dir) return null;

  return (
    <button
      onClick={() => invoke("open_folder", { path: dir })}
      title="Open folder"
      className="flex items-center gap-1.5 px-2 py-1 rounded text-xs text-gray-400 hover:text-accent hover:bg-surface-2 transition-colors"
    >
      <span>Open</span>
      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/><line x1="12" y1="11" x2="12" y2="17"/><polyline points="9 14 12 11 15 14"/></svg>
    </button>
  );
}

export default function App() {
  const [activeView, setActiveView] = useState<View>("chat");
  const [dataInitialTab, setDataInitialTab] = useState<"sessions" | "flow-logs" | "events" | "custom" | undefined>(undefined);
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
            onClick={() => { setDataInitialTab(undefined); setActiveView(tab.id); }}
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
        {activeView === "chat" ? (
          <span className="text-xs text-gray-500">
            {backend.personaName} | {backend.modelName}
          </span>
        ) : (
          <OpenFolderButton view={activeView} snapshot={backend.snapshot} />
        )}
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-hidden">
        {activeView === "chat" && (
          <ChatView
            backend={backend}
            inferenceConfigured={backend.inferenceConfigured}
            onNavigateSettings={() => setActiveView("settings")}
          />
        )}
        {activeView === "skills" && <SkillsView snapshot={backend.snapshot} />}
        {activeView === "personas" && <PersonasView snapshot={backend.snapshot} />}
        {activeView === "data" && (
          <DataView
            sessions={backend.sessions}
            viewingSession={backend.viewingSession}
            flowLogs={backend.flowLogs}
            onLoadSession={backend.loadSession}
            onDeleteSession={backend.deleteSession}
            onResumeSession={(id) => {
              backend.loadSession(id);
              setActiveView("chat");
            }}
            onLoadFlowLogs={backend.loadFlowLogs}
            internalEvents={backend.internalEvents}
            onLoadEventsLog={backend.loadEventsLog}
            onListCustomFiles={backend.listCustomFiles}
            onReadCustomFile={backend.readCustomFile}
            onWriteCustomFile={backend.writeCustomFile}
            initialTab={dataInitialTab}
          />
        )}
        {activeView === "flows" && (
          <SchedulingView
            flows={backend.flows}
            editingFlow={backend.editingFlow}
            flowTemplates={backend.flowTemplates}
            onSaveFlow={backend.saveFlow}
            onLoadFlow={backend.loadFlow}
            onDeleteFlow={backend.deleteFlow}
            onToggleFlowEnabled={backend.toggleFlowEnabled}
            onRunFlowOnce={backend.runFlowOnce}
            onListFlows={backend.listFlows}
            onClearEditingFlow={backend.clearEditingFlow}
            onListFlowTemplates={backend.listFlowTemplates}
            onImportFlowTemplate={backend.importIntegrationFlow}
          />
        )}
        {activeView === "api-keys" && (
          <ApiKeysView
            snapshot={backend.snapshot}
            onAdd={backend.addApiKey}
            onDelete={backend.deleteApiKey}
            onInstall={backend.installIntegration}
            onUninstall={backend.uninstallIntegration}
            onImportFlow={backend.importIntegrationFlow}
            onNavigatePacks={() => setActiveView("packs")}
          />
        )}
        {activeView === "packs" && (
          <PacksView
            packs={backend.remotePacks}
            loading={backend.packsLoading}
            message={backend.packsMessage}
            onListPacks={backend.listPacks}
            onInstall={backend.installPack}
            onUninstall={backend.uninstallPack}
          />
        )}
        {activeView === "gateway" && (
          <GatewayView
            channels={backend.channels}
            channelSettings={backend.channelSettings}
            onCreateChannel={backend.createChannel}
            onDeleteChannel={backend.deleteChannel}
            onStartChannel={backend.startChannel}
            onStopChannel={backend.stopChannel}
            onUpdateSetting={backend.updateChannelSetting}
            onLoadSettings={backend.loadChannelSettings}
            onListChannels={backend.listChannels}
          />
        )}
        {activeView === "skill-tests" && (
          <SkillTestsView
            skillTests={backend.skillTests}
            skillTestRunning={backend.skillTestRunning}
            skillTestReport={backend.skillTestReport}
            skillTestPartialResults={backend.skillTestPartialResults}
            skillTestCurrentTest={backend.skillTestCurrentTest}
            skillTestSteps={backend.skillTestSteps}
            onListSkillTests={backend.listSkillTests}
            onSaveSkillTest={backend.saveSkillTest}
            onDeleteSkillTest={backend.deleteSkillTest}
            onRunSkillTest={backend.runSkillTest}
          />
        )}
        {activeView === "settings" && (
          <SettingsView
            snapshot={backend.snapshot}
            currentModel={backend.modelName}
            onSwitchModel={backend.switchModel}
            onAddApiKey={backend.addApiKey}
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
        {backend.runningFlows > 0 && (
          <>
            <span className="text-gray-600">|</span>
            <button
              onClick={() => { setDataInitialTab("flow-logs"); setActiveView("data"); }}
              className="text-yellow-400 hover:text-yellow-300 transition-colors"
            >
              ⟳ {backend.runningFlows} flow{backend.runningFlows !== 1 ? "s" : ""} running
            </button>
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
