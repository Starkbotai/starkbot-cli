import { useState } from "react";
import type { SessionSummary, ChatSession } from "../types";

interface Props {
  sessions: SessionSummary[];
  viewingSession: ChatSession | null;
  onLoadSession: (id: string) => void;
  onDeleteSession: (id: string) => void;
}

export default function DataView({ sessions, viewingSession, onLoadSession, onDeleteSession }: Props) {
  const [selectedIdx, setSelectedIdx] = useState(0);

  return (
    <div className="flex h-full">
      {/* Pill sidebar */}
      <div className="w-44 border-r border-surface-3 p-3">
        <div className="px-3 py-1.5 rounded bg-accent/20 text-accent text-sm font-medium">
          Chat Sessions
        </div>
      </div>

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
              <button
                onClick={(e) => { e.stopPropagation(); onDeleteSession(s.id); }}
                className="mt-1 text-[10px] text-red-400 hover:text-red-300"
              >
                Delete
              </button>
            </div>
          ))
        )}
      </div>

      {/* Detail pane */}
      <div className="flex-1 overflow-y-auto p-4">
        {viewingSession ? (
          <div>
            <h2 className="text-lg font-semibold text-gray-200 mb-1">{viewingSession.title}</h2>
            <p className="text-xs text-gray-500 mb-4">
              {viewingSession.persona} | {viewingSession.created_at.slice(0, 10)} | {viewingSession.messages.length} messages
            </p>
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
    </div>
  );
}
