import { useState, useRef, useEffect } from "react";
import type { ChatMessage } from "../types";

interface PendingApproval {
  request_id: string;
  tool_name: string;
  args_display: string;
}

interface ChatBackend {
  messages: ChatMessage[];
  agentBusy: boolean;
  pendingApproval: PendingApproval | null;
  sendMessage: (content: string) => Promise<void>;
  approvalResponse: (requestId: string, approved: boolean) => Promise<void>;
}

function MessageBubble({ msg }: { msg: ChatMessage }) {
  const roleStyles: Record<string, string> = {
    user: "bg-accent/10 border-accent/30",
    assistant: "bg-surface-2 border-surface-3",
    thinking: "bg-surface-2/50 border-surface-3/50 opacity-60",
    tool: "bg-yellow-900/20 border-yellow-700/30",
    error: "bg-red-900/20 border-red-700/30",
  };

  const roleLabels: Record<string, { text: string; color: string }> = {
    user: { text: "You", color: "text-accent-light" },
    assistant: { text: "Agent", color: "text-cyan-400" },
    thinking: { text: "Thinking", color: "text-gray-500" },
    tool: { text: "Tool", color: "text-yellow-400" },
    error: { text: "Error", color: "text-red-400" },
  };

  const style = roleStyles[msg.role] || roleStyles.assistant;
  const label = roleLabels[msg.role] || roleLabels.assistant;

  return (
    <div className={`rounded-lg border px-4 py-3 ${style}`}>
      <div className={`text-xs font-semibold mb-1 ${label.color}`}>{label.text}</div>
      <div className="text-sm text-gray-200 whitespace-pre-wrap break-words">{msg.content}</div>
    </div>
  );
}

function ApprovalPrompt({
  approval,
  onRespond,
}: {
  approval: PendingApproval;
  onRespond: (approved: boolean) => void;
}) {
  return (
    <div className="mx-4 mb-3 p-4 rounded-lg border border-yellow-600/50 bg-yellow-900/20">
      <div className="text-sm font-semibold text-yellow-400 mb-2">Approve Tool Call?</div>
      <div className="text-sm text-white font-mono mb-1">{approval.tool_name}</div>
      <pre className="text-xs text-gray-400 mb-3 whitespace-pre-wrap max-h-24 overflow-y-auto">
        {approval.args_display}
      </pre>
      <div className="flex gap-2">
        <button
          onClick={() => onRespond(true)}
          className="px-4 py-1.5 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors"
        >
          Approve
        </button>
        <button
          onClick={() => onRespond(false)}
          className="px-4 py-1.5 bg-red-600 hover:bg-red-500 text-white text-sm rounded transition-colors"
        >
          Deny
        </button>
      </div>
    </div>
  );
}

export default function ChatView({ backend }: { backend: ChatBackend }) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [backend.messages]);

  useEffect(() => {
    if (!backend.agentBusy && !backend.pendingApproval) {
      inputRef.current?.focus();
    }
  }, [backend.agentBusy, backend.pendingApproval]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const msg = input.trim();
    if (!msg || backend.agentBusy) return;
    setInput("");
    await backend.sendMessage(msg);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
        {backend.messages.map((msg, i) => (
          <MessageBubble key={i} msg={msg} />
        ))}
        <div ref={messagesEndRef} />
      </div>

      {/* Approval prompt */}
      {backend.pendingApproval && (
        <ApprovalPrompt
          approval={backend.pendingApproval}
          onRespond={(approved) =>
            backend.approvalResponse(backend.pendingApproval!.request_id, approved)
          }
        />
      )}

      {/* Input */}
      <form onSubmit={handleSubmit} className="px-4 py-3 border-t border-surface-3">
        <div className="flex gap-2">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={backend.agentBusy ? "Agent is thinking..." : "Type a message..."}
            disabled={backend.agentBusy}
            className="flex-1 px-4 py-2 bg-surface-2 border border-surface-3 rounded-lg text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-accent/50 disabled:opacity-50"
          />
          <button
            type="submit"
            disabled={backend.agentBusy || !input.trim()}
            className="px-4 py-2 bg-accent hover:bg-accent-dim text-white text-sm rounded-lg transition-colors disabled:opacity-30"
          >
            Send
          </button>
        </div>
      </form>
    </div>
  );
}
