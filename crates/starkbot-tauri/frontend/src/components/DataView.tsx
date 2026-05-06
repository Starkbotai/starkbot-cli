import { useState, useEffect } from "react";
import type { SessionSummary, ChatSession, FlowLogEntry } from "../types";

type DataTab = "sessions" | "flow-logs";

interface Props {
  sessions: SessionSummary[];
  viewingSession: ChatSession | null;
  flowLogs: FlowLogEntry[];
  onLoadSession: (id: string) => void;
  onDeleteSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  onLoadFlowLogs: () => void;
}

export default function DataView({
  sessions,
  viewingSession,
  flowLogs,
  onLoadSession,
  onDeleteSession,
  onResumeSession,
  onLoadFlowLogs,
}: Props) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [tab, setTab] = useState<DataTab>("sessions");

  useEffect(() => {
    if (tab === "flow-logs") {
      onLoadFlowLogs();
    }
  }, [tab]);

  return (
    <div className="flex h-full">
      {/* Pill sidebar */}
      <div className="w-44 border-r border-surface-3 p-3 space-y-1">
        <button
          onClick={() => setTab("sessions")}
          className={`w-full text-left px-3 py-1.5 rounded text-sm font-medium transition-colors ${
            tab === "sessions" ? "bg-accent/20 text-accent" : "text-gray-400 hover:text-gray-200 hover:bg-surface-2"
          }`}
        >
          Chat Sessions
        </button>
        <button
          onClick={() => setTab("flow-logs")}
          className={`w-full text-left px-3 py-1.5 rounded text-sm font-medium transition-colors ${
            tab === "flow-logs" ? "bg-accent/20 text-accent" : "text-gray-400 hover:text-gray-200 hover:bg-surface-2"
          }`}
        >
          Flow Logs
        </button>
      </div>

      {/* Content area */}
      {tab === "sessions" && (
        <>
          {/* Session list */}
          <div className="w-[35%] border-r border-surface-3 overflow-y-auto">
            <div className="p-2 text-xs text-gray-500 border-b border-surface-3">
              {sessions.length} session{sessions.length !== 1 ? "s" : ""}
            </div>
            {sessions.length === 0 ? (
              <div className="p-4 text-sm text-gray-500">
                No saved sessions. Start a chat to create one.
              </div>
            ) : (
              sessions.map((s, i) => (
                <div
                  key={s.id}
                  onClick={() => { setSelectedIdx(i); onLoadSession(s.id); }}
                  className={`px-3 py-2 cursor-pointer border-b border-surface-2 hover:bg-surface-2 transition-colors ${
                    i === selectedIdx ? "bg-surface-2 border-l-2 border-l-accent" : ""
                  }`}
                >
                  <div className="text-sm text-gray-200 truncate">{s.title}</div>
                  <div className="flex items-center gap-2 text-xs text-gray-500 mt-0.5">
                    <span>{s.persona}</span>
                    <span>|</span>
                    <span>{s.message_count} msgs</span>
                    <span>|</span>
                    <span>{s.created_at.slice(0, 10)}</span>
                  </div>
                </div>
              ))
            )}
          </div>

          {/* Detail pane */}
          <div className="flex-1 overflow-y-auto p-4">
            {viewingSession ? (
              <div>
                <h2 className="text-lg font-semibold text-gray-200 mb-1">{viewingSession.title}</h2>
                <p className="text-xs text-gray-500 mb-3">
                  {viewingSession.persona} | {viewingSession.created_at.slice(0, 10)} | {viewingSession.messages.length} messages
                </p>
                <div className="flex gap-2 mb-4">
                  <button
                    onClick={() => onResumeSession(viewingSession.id)}
                    className="px-3 py-1 text-sm rounded bg-accent/20 text-accent hover:bg-accent/30"
                  >
                    Resume
                  </button>
                  <button
                    onClick={() => onDeleteSession(viewingSession.id)}
                    className="px-3 py-1 text-sm rounded bg-red-900/30 text-red-400 hover:bg-red-900/50"
                  >
                    Delete
                  </button>
                </div>
                <div className="space-y-3">
                  {viewingSession.messages.map((msg, i) => (
                    <div key={i} className={`text-sm ${msg.role === "user" ? "text-green-400" : "text-gray-300"}`}>
                      <span className="font-bold text-xs mr-2">
                        {msg.role === "user" ? "[you]" : "[agent]"}
                      </span>
                      <span className="whitespace-pre-wrap">{msg.content}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="text-sm text-gray-500">Select a session to view its messages.</div>
            )}
          </div>
        </>
      )}

      {tab === "flow-logs" && (
        <div className="flex-1 overflow-y-auto p-4">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-200">Flow Logs</h2>
            <button
              onClick={onLoadFlowLogs}
              className="px-2 py-1 text-xs rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
            >
              Refresh
            </button>
          </div>
          {flowLogs.length === 0 ? (
            <div className="text-sm text-gray-500">No flow activity recorded yet.</div>
          ) : (
            <div className="space-y-1">
              {[...flowLogs].reverse().map((log, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 px-3 py-2 rounded border border-surface-3 bg-surface-1 text-sm"
                >
                  <span className="text-[10px] text-gray-500 font-mono whitespace-nowrap">
                    {log.timestamp.slice(0, 19).replace("T", " ")}
                  </span>
                  <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${
                    log.action === "enabled" ? "bg-green-900/40 text-green-400" :
                    log.action === "disabled" ? "bg-gray-800 text-gray-400" :
                    log.action === "deleted" ? "bg-red-900/40 text-red-400" :
                    "bg-blue-900/40 text-blue-400"
                  }`}>
                    {log.action}
                  </span>
                  <span className="text-gray-200 truncate">{log.flow_name}</span>
                  {log.detail && (
                    <span className="text-gray-500 text-xs">{log.detail}</span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
