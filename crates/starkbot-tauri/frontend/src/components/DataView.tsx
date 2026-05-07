import { useState, useEffect, useMemo } from "react";
import type { SessionSummary, ChatSession, FlowLogEntry, CustomFileEntry, InternalEvent } from "../types";

type DataTab = "sessions" | "flow-logs" | "events" | "custom";

interface Props {
  sessions: SessionSummary[];
  viewingSession: ChatSession | null;
  flowLogs: FlowLogEntry[];
  onLoadSession: (id: string) => void;
  onDeleteSession: (id: string) => void;
  onResumeSession: (id: string) => void;
  onLoadFlowLogs: () => void;
  internalEvents: InternalEvent[];
  onLoadEventsLog: () => void;
  onListCustomFiles: () => Promise<CustomFileEntry[]>;
  onReadCustomFile: (path: string) => Promise<string>;
  onWriteCustomFile: (path: string, content: string) => Promise<void>;
}

export default function DataView({
  sessions,
  viewingSession,
  flowLogs,
  onLoadSession,
  onDeleteSession,
  onResumeSession,
  onLoadFlowLogs,
  internalEvents,
  onLoadEventsLog,
  onListCustomFiles,
  onReadCustomFile,
  onWriteCustomFile,
}: Props) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [tab, setTab] = useState<DataTab>("sessions");

  useEffect(() => {
    if (tab === "flow-logs") {
      onLoadFlowLogs();
    } else if (tab === "events") {
      onLoadEventsLog();
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
        <button
          onClick={() => setTab("events")}
          className={`w-full text-left px-3 py-1.5 rounded text-sm font-medium transition-colors ${
            tab === "events" ? "bg-accent/20 text-accent" : "text-gray-400 hover:text-gray-200 hover:bg-surface-2"
          }`}
        >
          Events
        </button>
        <button
          onClick={() => setTab("custom")}
          className={`w-full text-left px-3 py-1.5 rounded text-sm font-medium transition-colors ${
            tab === "custom" ? "bg-accent/20 text-accent" : "text-gray-400 hover:text-gray-200 hover:bg-surface-2"
          }`}
        >
          Custom
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
        <FlowLogsPanel flowLogs={flowLogs} onLoadFlowLogs={onLoadFlowLogs} />
      )}

      {tab === "events" && (
        <EventsLogPanel events={internalEvents} onRefresh={onLoadEventsLog} />
      )}

      {tab === "custom" && (
        <CustomFilesPanel
          onListFiles={onListCustomFiles}
          onReadFile={onReadCustomFile}
          onWriteFile={onWriteCustomFile}
        />
      )}
    </div>
  );
}

// --- Events Log Panel ---

function EventsLogPanel({ events, onRefresh }: { events: InternalEvent[]; onRefresh: () => void }) {
  // Show newest first
  const reversed = useMemo(() => [...events].reverse(), [events]);

  const kindColor = (kind: string) => {
    switch (kind) {
      case "pulse": return "bg-gray-800 text-gray-500";
      case "run_flow": return "bg-blue-900/40 text-blue-400";
      case "flow_completed": return "bg-green-900/40 text-green-400";
      case "flow_error": return "bg-red-900/40 text-red-400";
      default: return "bg-gray-800 text-gray-400";
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-200">Events Log</h2>
        <button
          onClick={onRefresh}
          className="px-2 py-1 text-xs rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
        >
          Refresh
        </button>
      </div>
      {reversed.length === 0 ? (
        <div className="text-sm text-gray-500">No events recorded yet. Events appear as the engine runs.</div>
      ) : (
        <div className="space-y-1">
          {reversed.map((evt, i) => (
            <div
              key={i}
              className="flex items-center gap-3 px-3 py-1.5 rounded border border-surface-3 bg-surface-1 text-sm"
            >
              <span className="text-[10px] text-gray-500 font-mono whitespace-nowrap">
                {evt.timestamp.slice(11, 19)}
              </span>
              <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${kindColor(evt.kind)}`}>
                {evt.kind}
              </span>
              {evt.payload && (
                <span className="text-gray-400 text-xs truncate">{evt.payload}</span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// --- Custom Files Panel ---

function CustomFilesPanel({
  onListFiles,
  onReadFile,
  onWriteFile,
}: {
  onListFiles: () => Promise<CustomFileEntry[]>;
  onReadFile: (path: string) => Promise<string>;
  onWriteFile: (path: string, content: string) => Promise<void>;
}) {
  const [files, setFiles] = useState<CustomFileEntry[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState("");
  const [editedContent, setEditedContent] = useState("");
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadFiles();
  }, []);

  const loadFiles = async () => {
    try {
      const entries = await onListFiles();
      setFiles(entries);
    } catch (e) {
      console.error("Failed to list custom files:", e);
    }
  };

  const handleSelect = async (path: string) => {
    setSelectedFile(path);
    setEditing(false);
    try {
      const content = await onReadFile(path);
      setFileContent(content);
      setEditedContent(content);
    } catch (e) {
      setFileContent(`Error reading file: ${e}`);
      setEditedContent("");
    }
  };

  const handleSave = async () => {
    if (!selectedFile) return;
    setSaving(true);
    try {
      await onWriteFile(selectedFile, editedContent);
      setFileContent(editedContent);
      setEditing(false);
    } catch (e) {
      console.error("Failed to save:", e);
    }
    setSaving(false);
  };

  const fileItems = files.filter((f) => !f.is_dir);

  return (
    <div className="flex flex-1 h-full">
      {/* File list */}
      <div className="w-[35%] border-r border-surface-3 overflow-y-auto">
        <div className="flex items-center justify-between p-2 border-b border-surface-3">
          <span className="text-xs text-gray-500">{fileItems.length} file{fileItems.length !== 1 ? "s" : ""}</span>
          <button
            onClick={loadFiles}
            className="px-2 py-1 text-xs rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
          >
            Refresh
          </button>
        </div>
        {fileItems.length === 0 ? (
          <div className="p-4 text-sm text-gray-500">
            No custom files yet. Install an integration with custom configs to populate this folder.
          </div>
        ) : (
          fileItems.map((f) => (
            <div
              key={f.path}
              onClick={() => handleSelect(f.path)}
              className={`px-3 py-2 cursor-pointer border-b border-surface-2 hover:bg-surface-2 transition-colors ${
                selectedFile === f.path ? "bg-surface-2 border-l-2 border-l-accent" : ""
              }`}
            >
              <div className="text-sm text-gray-200 truncate font-mono">{f.path}</div>
            </div>
          ))
        )}
      </div>

      {/* File content */}
      <div className="flex-1 overflow-y-auto p-4 flex flex-col">
        {selectedFile ? (
          <>
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-gray-200 font-mono">{selectedFile}</h3>
              <div className="flex gap-2">
                {editing ? (
                  <>
                    <button
                      onClick={handleSave}
                      disabled={saving}
                      className="px-3 py-1 text-xs rounded bg-green-600 hover:bg-green-500 text-white disabled:opacity-50"
                    >
                      {saving ? "Saving..." : "Save"}
                    </button>
                    <button
                      onClick={() => { setEditing(false); setEditedContent(fileContent); }}
                      className="px-3 py-1 text-xs rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
                    >
                      Cancel
                    </button>
                  </>
                ) : (
                  <button
                    onClick={() => setEditing(true)}
                    className="px-3 py-1 text-xs rounded bg-accent/20 text-accent hover:bg-accent/30"
                  >
                    Edit
                  </button>
                )}
              </div>
            </div>
            {editing ? (
              <textarea
                value={editedContent}
                onChange={(e) => setEditedContent(e.target.value)}
                className="flex-1 w-full bg-surface-2 border border-surface-3 rounded p-3 text-sm text-gray-200 font-mono resize-none focus:outline-none focus:border-accent/50"
              />
            ) : (
              <pre className="flex-1 bg-surface-2 border border-surface-3 rounded p-3 text-sm text-gray-200 font-mono overflow-auto whitespace-pre-wrap">
                {fileContent}
              </pre>
            )}
          </>
        ) : (
          <div className="text-sm text-gray-500">Select a file to view or edit its contents.</div>
        )}
      </div>
    </div>
  );
}

// --- Grouped Flow Logs ---

type RunGroup = {
  run_id: string;
  flow_name: string;
  entries: FlowLogEntry[];
  startTime: string;
  endTime: string;
  success: boolean;
};

type LogItem = { type: "run"; group: RunGroup } | { type: "admin"; entry: FlowLogEntry };

function FlowLogsPanel({ flowLogs, onLoadFlowLogs }: { flowLogs: FlowLogEntry[]; onLoadFlowLogs: () => void }) {
  const [expandedRuns, setExpandedRuns] = useState<Set<string>>(new Set());

  const items: LogItem[] = useMemo(() => {
    // Group entries by run_id; entries without run_id are admin actions
    const runMap = new Map<string, FlowLogEntry[]>();
    const adminEntries: { idx: number; entry: FlowLogEntry }[] = [];
    const runFirstSeen = new Map<string, number>(); // run_id -> last entry index (for sorting)

    flowLogs.forEach((entry, idx) => {
      if (entry.run_id) {
        if (!runMap.has(entry.run_id)) {
          runMap.set(entry.run_id, []);
          runFirstSeen.set(entry.run_id, idx);
        }
        runMap.get(entry.run_id)!.push(entry);
        // Update to latest index for this run
        runFirstSeen.set(entry.run_id, idx);
      } else {
        adminEntries.push({ idx, entry });
      }
    });

    // Build run groups
    const runGroups: { idx: number; item: LogItem }[] = [];
    for (const [run_id, entries] of runMap) {
      const startTime = entries[0].timestamp;
      const endTime = entries[entries.length - 1].timestamp;
      const executedEntry = entries.find(e => e.action === "executed");
      const success = executedEntry ? executedEntry.detail.includes("successfully") : false;
      runGroups.push({
        idx: runFirstSeen.get(run_id)!,
        item: {
          type: "run",
          group: { run_id, flow_name: entries[0].flow_name, entries, startTime, endTime, success },
        },
      });
    }

    // Merge and sort reverse-chronological (highest index first)
    const all: { idx: number; item: LogItem }[] = [
      ...runGroups,
      ...adminEntries.map(a => ({ idx: a.idx, item: { type: "admin" as const, entry: a.entry } })),
    ];
    all.sort((a, b) => b.idx - a.idx);
    return all.map(a => a.item);
  }, [flowLogs]);

  const toggleExpand = (runId: string) => {
    setExpandedRuns(prev => {
      const next = new Set(prev);
      if (next.has(runId)) next.delete(runId); else next.add(runId);
      return next;
    });
  };

  return (
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
      {items.length === 0 ? (
        <div className="text-sm text-gray-500">No flow activity recorded yet.</div>
      ) : (
        <div className="space-y-2">
          {items.map((item, i) => {
            if (item.type === "admin") {
              const log = item.entry;
              return (
                <div
                  key={`admin-${i}`}
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
                  {log.detail && <span className="text-gray-500 text-xs">{log.detail}</span>}
                </div>
              );
            }

            const { group } = item;
            const expanded = expandedRuns.has(group.run_id);
            const duration = computeDuration(group.startTime, group.endTime);

            return (
              <div
                key={`run-${group.run_id}`}
                className="rounded border border-surface-3 bg-surface-1 overflow-hidden"
              >
                <button
                  onClick={() => toggleExpand(group.run_id)}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-surface-2 transition-colors text-left"
                >
                  <span className="text-[10px] text-gray-500 font-mono whitespace-nowrap">
                    {group.startTime.slice(0, 19).replace("T", " ")}
                  </span>
                  <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${
                    group.success ? "bg-green-900/40 text-green-400" : "bg-red-900/40 text-red-400"
                  }`}>
                    {group.success ? "success" : "error"}
                  </span>
                  <span className="text-gray-200 truncate font-medium">{group.flow_name}</span>
                  {duration && <span className="text-gray-500 text-xs">{duration}</span>}
                  <span className="ml-auto text-gray-500 text-xs">{expanded ? "▾" : "▸"} {group.entries.length} entries</span>
                </button>
                {expanded && (
                  <div className="border-t border-surface-3 px-3 py-2 space-y-1">
                    {group.entries.map((entry, j) => (
                      <div key={j} className="flex items-start gap-2 text-xs text-gray-400">
                        <span className="text-[10px] text-gray-600 font-mono whitespace-nowrap">
                          {entry.timestamp.slice(11, 19)}
                        </span>
                        <span className="text-blue-400">{entry.action}</span>
                        {entry.detail && <span className={`${entry.detail.startsWith("error:") ? "text-red-400" : "text-gray-500"} break-all`}>{entry.detail}</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function computeDuration(start: string, end: string): string | null {
  try {
    const s = new Date(start).getTime();
    const e = new Date(end).getTime();
    const diff = e - s;
    if (diff <= 0 || isNaN(diff)) return null;
    if (diff < 1000) return `${diff}ms`;
    const secs = Math.round(diff / 1000);
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    const remSecs = secs % 60;
    return `${mins}m ${remSecs}s`;
  } catch {
    return null;
  }
}
