import { useState, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import type { ChatMessage } from "../types";

interface PendingApproval {
  request_id: string;
  tool_name: string;
  args_display: string;
}

interface SlashCommandDef {
  name: string;
  description: string;
}

const SLASH_COMMANDS: SlashCommandDef[] = [
  { name: "new", description: "Start a new chat session" },
  { name: "clear", description: "Clear chat history" },
  { name: "tokens", description: "Show token usage" },
  { name: "help", description: "Show available commands" },
];

interface ChatBackend {
  messages: ChatMessage[];
  agentBusy: boolean;
  pendingApproval: PendingApproval | null;
  sendMessage: (content: string) => Promise<void>;
  approvalResponse: (requestId: string, approved: boolean) => Promise<void>;
  slashCommand: (command: string) => Promise<void>;
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
      <div className="text-sm text-gray-200 prose prose-invert prose-sm max-w-none break-words">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            a({ href, children, ...props }) {
              return (
                <a
                  {...props}
                  href={href}
                  className="text-accent-light underline hover:text-accent cursor-pointer"
                  onClick={(e) => {
                    e.preventDefault();
                    if (href) invoke("open_url", { url: href });
                  }}
                >
                  {children}
                </a>
              );
            },
            code({ className, children, ...props }) {
              const match = /language-(\w+)/.exec(className || "");
              const inline = !match && !String(children).includes("\n");
              return inline ? (
                <code className="px-1.5 py-0.5 rounded bg-surface-3 text-accent-light text-xs font-mono" {...props}>
                  {children}
                </code>
              ) : (
                <SyntaxHighlighter
                  style={oneDark}
                  language={match?.[1] || "text"}
                  PreTag="div"
                  customStyle={{ margin: "0.5rem 0", borderRadius: "0.5rem", fontSize: "0.75rem" }}
                >
                  {String(children).replace(/\n$/, "")}
                </SyntaxHighlighter>
              );
            },
          }}
        >
          {msg.content}
        </ReactMarkdown>
      </div>
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

function SlashCommandMenu({
  commands,
  selectedIndex,
  onSelect,
}: {
  commands: SlashCommandDef[];
  selectedIndex: number;
  onSelect: (cmd: SlashCommandDef) => void;
}) {
  return (
    <div className="absolute bottom-full left-0 right-0 mb-1 bg-surface-2 border border-surface-3 rounded-lg shadow-lg overflow-hidden z-10">
      {commands.map((cmd, i) => (
        <button
          key={cmd.name}
          onClick={() => onSelect(cmd)}
          className={`w-full text-left px-4 py-2.5 flex items-center gap-3 transition-colors ${
            i === selectedIndex ? "bg-accent/20 text-white" : "text-gray-300 hover:bg-surface-3"
          }`}
        >
          <span className="text-sm font-mono text-accent-light">/{cmd.name}</span>
          <span className="text-xs text-gray-500">{cmd.description}</span>
        </button>
      ))}
    </div>
  );
}

export default function ChatView({
  backend,
  inferenceConfigured = true,
  onNavigateSettings,
}: {
  backend: ChatBackend;
  inferenceConfigured?: boolean;
  onNavigateSettings?: () => void;
}) {
  const [input, setInput] = useState("");
  const [selectedCmd, setSelectedCmd] = useState(0);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Show menu when input starts with /
  const showMenu = input.startsWith("/") && !backend.agentBusy;
  const query = input.slice(1).toLowerCase();
  const filteredCommands = useMemo(
    () => (showMenu ? SLASH_COMMANDS.filter((c) => c.name.startsWith(query)) : []),
    [showMenu, query],
  );

  useEffect(() => {
    setSelectedCmd(0);
  }, [input]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [backend.messages]);

  useEffect(() => {
    if (!backend.agentBusy && !backend.pendingApproval) {
      inputRef.current?.focus();
    }
  }, [backend.agentBusy, backend.pendingApproval]);

  const runSlashCommand = (name: string) => {
    setInput("");
    backend.slashCommand(`/${name}`);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const msg = input.trim();
    if (!msg || backend.agentBusy) return;

    // Slash command
    if (msg.startsWith("/")) {
      const cmdName = msg.slice(1).split(/\s/)[0].toLowerCase();
      const found = SLASH_COMMANDS.find((c) => c.name === cmdName);
      if (found) {
        runSlashCommand(found.name);
        return;
      }
      // Still send to backend for unknown commands (it will show an error)
      setInput("");
      await backend.slashCommand(msg);
      return;
    }

    setInput("");
    await backend.sendMessage(msg);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!showMenu || filteredCommands.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedCmd((prev) => (prev + 1) % filteredCommands.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedCmd((prev) => (prev - 1 + filteredCommands.length) % filteredCommands.length);
    } else if (e.key === "Tab") {
      e.preventDefault();
      // Tab autocompletes the command name
      setInput(`/${filteredCommands[selectedCmd].name}`);
    } else if (e.key === "Enter" && showMenu && filteredCommands.length > 0 && input !== `/${filteredCommands[selectedCmd].name}`) {
      // If the menu is visible and it's not an exact match yet, select the highlighted command
      e.preventDefault();
      runSlashCommand(filteredCommands[selectedCmd].name);
    }
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

      {/* Input or setup banner */}
      {!inferenceConfigured ? (
        <div className="px-4 py-4 border-t border-surface-3">
          <div className="flex items-center gap-3 p-4 rounded-lg bg-yellow-900/20 border border-yellow-700/30">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-yellow-400 flex-shrink-0">
              <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <div className="flex-1">
              <div className="text-sm font-semibold text-yellow-400">Inference not configured</div>
              <div className="text-xs text-gray-400 mt-0.5">Add your OpenAI API key to start chatting</div>
            </div>
            {onNavigateSettings && (
              <button
                onClick={onNavigateSettings}
                className="px-4 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors"
              >
                Configure Inference
              </button>
            )}
          </div>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className="px-4 py-3 border-t border-surface-3 relative">
          {/* Slash command autocomplete */}
          {showMenu && filteredCommands.length > 0 && (
            <SlashCommandMenu
              commands={filteredCommands}
              selectedIndex={selectedCmd}
              onSelect={(cmd) => runSlashCommand(cmd.name)}
            />
          )}
          <div className="flex gap-2">
            <input
              ref={inputRef}
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={backend.agentBusy ? "Agent is thinking..." : "Type a message... (/ for commands)"}
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
      )}
    </div>
  );
}
