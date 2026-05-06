import { useState } from "react";
import type { ScheduledTaskSummary, Schedule, FlowDefinition } from "../types";

interface Props {
  tasks: ScheduledTaskSummary[];
  onCreate: (name: string, schedule: Schedule, flow: FlowDefinition) => void;
  onDelete: (taskId: string) => void;
  onToggle: (taskId: string) => void;
}

function formatSchedule(s: Schedule): string {
  if (s.type === "every_minutes") return `every ${s.value}m`;
  if (s.type === "every_hours") return `every ${s.value}h`;
  return "unknown";
}

export default function SchedulingView({ tasks, onCreate, onDelete, onToggle }: Props) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [schedType, setSchedType] = useState<"every_minutes" | "every_hours">("every_minutes");
  const [schedValue, setSchedValue] = useState("5");

  const handleCreate = () => {
    if (!newName.trim()) return;
    const schedule: Schedule = { type: schedType, value: parseInt(schedValue) || 5 };
    const flow: FlowDefinition = { nodes: [], edges: [] };
    onCreate(newName.trim(), schedule, flow);
    setNewName("");
    setSchedValue("5");
    setShowCreate(false);
  };

  const selected = selectedIdx < tasks.length ? tasks[selectedIdx] : null;

  return (
    <div className="flex h-full">
      {/* Task list */}
      <div className="w-[40%] border-r border-surface-3 flex flex-col">
        <div className="flex items-center justify-between p-2 border-b border-surface-3">
          <span className="text-xs text-gray-500">{tasks.length} task{tasks.length !== 1 ? "s" : ""}</span>
          <button
            onClick={() => setShowCreate(!showCreate)}
            className="px-2 py-0.5 text-xs rounded bg-accent/20 text-accent hover:bg-accent/30"
          >
            + New
          </button>
        </div>

        {showCreate && (
          <div className="p-3 border-b border-surface-3 bg-surface-1 space-y-2">
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="Task name"
              className="w-full px-2 py-1 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none"
            />
            <div className="flex gap-2">
              <select
                value={schedType}
                onChange={(e) => setSchedType(e.target.value as "every_minutes" | "every_hours")}
                className="flex-1 px-2 py-1 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200"
              >
                <option value="every_minutes">Minutes</option>
                <option value="every_hours">Hours</option>
              </select>
              <input
                value={schedValue}
                onChange={(e) => setSchedValue(e.target.value)}
                type="number"
                min="1"
                className="w-20 px-2 py-1 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200"
              />
            </div>
            <button
              onClick={handleCreate}
              className="w-full px-2 py-1 text-sm rounded bg-accent text-white hover:bg-accent/80"
            >
              Create
            </button>
          </div>
        )}

        <div className="flex-1 overflow-y-auto">
          {tasks.length === 0 ? (
            <div className="p-4 text-sm text-gray-500">
              No scheduled tasks. Click "+ New" to create one.
            </div>
          ) : (
            tasks.map((t, i) => (
              <div
                key={t.id}
                onClick={() => setSelectedIdx(i)}
                className={`px-3 py-2 cursor-pointer border-b border-surface-2 hover:bg-surface-2 transition-colors ${
                  i === selectedIdx ? "bg-surface-2 border-l-2 border-l-accent" : ""
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-200">{t.name}</span>
                  <span className={`text-[10px] px-1.5 py-0.5 rounded ${t.enabled ? "bg-green-900/40 text-green-400" : "bg-red-900/40 text-red-400"}`}>
                    {t.enabled ? "ON" : "OFF"}
                  </span>
                </div>
                <div className="text-xs text-gray-500 mt-0.5">
                  {formatSchedule(t.schedule)} | {t.node_count} nodes
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Detail */}
      <div className="flex-1 overflow-y-auto p-4">
        {selected ? (
          <div>
            <h2 className="text-lg font-semibold text-gray-200 mb-1">{selected.name}</h2>
            <div className="flex items-center gap-3 text-sm mb-4">
              <span className="text-yellow-400">{formatSchedule(selected.schedule)}</span>
              <span className={selected.enabled ? "text-green-400" : "text-red-400"}>
                {selected.enabled ? "Enabled" : "Disabled"}
              </span>
              <span className="text-gray-500">{selected.node_count} nodes</span>
              <span className="text-gray-500">{selected.created_at.slice(0, 10)}</span>
            </div>
            <div className="flex gap-2 mb-4">
              <button
                onClick={() => onToggle(selected.id)}
                className="px-3 py-1 text-sm rounded bg-surface-2 text-gray-300 hover:bg-surface-3"
              >
                {selected.enabled ? "Disable" : "Enable"}
              </button>
              <button
                onClick={() => onDelete(selected.id)}
                className="px-3 py-1 text-sm rounded bg-red-900/30 text-red-400 hover:bg-red-900/50"
              >
                Delete
              </button>
            </div>
            <div className="text-xs text-gray-500">
              <p>Flow nodes and edges will appear here once the task has a flow defined.</p>
              <p className="mt-1">Node count: {selected.node_count}</p>
            </div>
          </div>
        ) : (
          <div className="text-sm text-gray-500">Select a task to view details.</div>
        )}
      </div>
    </div>
  );
}
