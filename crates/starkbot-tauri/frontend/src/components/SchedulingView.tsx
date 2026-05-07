import { useState, useEffect } from "react";
import type { SavedFlow, FlowSummary, FlowTemplateInfo } from "../types";
import FlowEditor from "./flow/FlowEditor";

interface Props {
  flows: FlowSummary[];
  editingFlow: SavedFlow | null;
  flowTemplates: FlowTemplateInfo[];
  onSaveFlow: (flow: SavedFlow) => void;
  onLoadFlow: (flowId: string) => void;
  onDeleteFlow: (flowId: string) => void;
  onToggleFlowEnabled: (flowId: string) => void;
  onRunFlowOnce: (flowId: string) => void;
  onListFlows: () => void;
  onClearEditingFlow: () => void;
  onListFlowTemplates: () => void;
  onImportFlowTemplate: (presetId: string) => void;
}

export default function FlowsView({
  flows,
  editingFlow,
  flowTemplates,
  onSaveFlow,
  onLoadFlow,
  onDeleteFlow,
  onToggleFlowEnabled,
  onRunFlowOnce,
  onListFlows,
  onClearEditingFlow,
  onListFlowTemplates,
  onImportFlowTemplate,
}: Props) {
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [showFlowEditor, setShowFlowEditor] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);

  // Load flows list on mount
  useEffect(() => {
    onListFlows();
  }, []);

  const selected = selectedIdx < flows.length ? flows[selectedIdx] : null;

  const handleNewFlow = () => {
    const now = new Date().toISOString();
    const id = crypto.randomUUID();
    const newFlow: SavedFlow = {
      id,
      name: "New Flow",
      flow: { nodes: [], edges: [] },
      created_at: now,
      updated_at: now,
      enabled: false,
    };
    onSaveFlow(newFlow);
    onLoadFlow(id);
    setShowFlowEditor(true);
  };

  const handleNewFromTemplate = () => {
    onListFlowTemplates();
    setShowTemplates(true);
  };

  const handleImportTemplate = (presetId: string) => {
    onImportFlowTemplate(presetId);
    setShowTemplates(false);
  };

  const handleEditFlow = () => {
    if (!selected) return;
    onLoadFlow(selected.id);
    setShowFlowEditor(true);
  };

  const handleFlowSave = (flow: SavedFlow) => {
    onSaveFlow(flow);
  };

  const handleFlowClose = () => {
    setShowFlowEditor(false);
    onClearEditingFlow();
    onListFlows();
  };

  // Show flow editor overlay
  if (showFlowEditor && editingFlow) {
    return (
      <FlowEditor
        flow={editingFlow}
        onSave={handleFlowSave}
        onClose={handleFlowClose}
      />
    );
  }

  return (
    <div className="flex h-full">
      {/* Flow list */}
      <div className="w-[40%] border-r border-surface-3 flex flex-col">
        <div className="flex items-center justify-between p-2 border-b border-surface-3">
          <span className="text-xs text-gray-500">{flows.length} flow{flows.length !== 1 ? "s" : ""}</span>
          <div className="flex gap-1">
            <button
              onClick={handleNewFromTemplate}
              className="px-2 py-0.5 text-xs rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
              title="Create flow from an integration template"
            >
              From Template
            </button>
            <button
              onClick={handleNewFlow}
              className="px-2 py-0.5 text-xs rounded bg-accent/20 text-accent hover:bg-accent/30"
            >
              + New
            </button>
          </div>
        </div>

        {/* Template picker dropdown */}
        {showTemplates && (
          <div className="border-b border-surface-3 bg-surface-1 p-3">
            {flowTemplates.length === 0 ? (
              <div className="text-xs text-gray-500">
                No templates available. Install an integration with a flow template first.
              </div>
            ) : (
              <div className="space-y-1">
                <div className="text-xs text-gray-500 mb-1">Import a flow template:</div>
                {flowTemplates.map((t) => (
                  <button
                    key={t.preset_id}
                    onClick={() => handleImportTemplate(t.preset_id)}
                    className="w-full text-left px-3 py-2 rounded border border-surface-3 hover:border-gray-500 bg-surface-2 transition-colors"
                  >
                    <div className="text-sm text-white">{t.template_name}</div>
                    <div className="text-[10px] text-gray-500">from {t.preset_name}</div>
                  </button>
                ))}
              </div>
            )}
            <button
              onClick={() => setShowTemplates(false)}
              className="mt-2 text-xs text-gray-500 hover:text-gray-300"
            >
              Cancel
            </button>
          </div>
        )}

        <div className="flex-1 overflow-y-auto">
          {flows.length === 0 ? (
            <div className="p-4 text-sm text-gray-500">
              No flows yet. Click "+ New" to create one.
            </div>
          ) : (
            flows.map((f, i) => (
              <div
                key={f.id}
                onClick={() => setSelectedIdx(i)}
                onDoubleClick={() => {
                  setSelectedIdx(i);
                  onLoadFlow(f.id);
                  setShowFlowEditor(true);
                }}
                className={`px-3 py-2 cursor-pointer border-b border-surface-2 hover:bg-surface-2 transition-colors ${
                  i === selectedIdx ? "bg-surface-2 border-l-2 border-l-accent" : ""
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-200">{f.name}</span>
                  {f.enabled ? (
                    <span className="px-1 py-0.5 text-[9px] rounded bg-green-900/40 text-green-400 border border-green-800/50">ON</span>
                  ) : (
                    <span className="px-1 py-0.5 text-[9px] rounded bg-gray-800 text-gray-500 border border-gray-700">OFF</span>
                  )}
                </div>
                <div className="text-xs text-gray-500 mt-0.5">
                  {f.node_count} nodes | {f.updated_at.slice(0, 10)}
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
              <span className="text-gray-500">{selected.node_count} nodes</span>
              <span className="text-gray-500">Created {selected.created_at.slice(0, 10)}</span>
              <span className="text-gray-500">Updated {selected.updated_at.slice(0, 10)}</span>
            </div>
            <div className="flex gap-2 mb-4">
              <button
                onClick={handleEditFlow}
                className="px-3 py-1 text-sm rounded bg-indigo-500/20 text-indigo-300 hover:bg-indigo-500/30"
              >
                Edit Flow
              </button>
              <button
                onClick={() => onRunFlowOnce(selected.id)}
                className="px-3 py-1 text-sm rounded bg-cyan-500/20 text-cyan-300 hover:bg-cyan-500/30"
              >
                Run Once
              </button>
              <button
                onClick={() => onToggleFlowEnabled(selected.id)}
                className={`px-3 py-1 text-sm rounded ${
                  selected.enabled
                    ? "bg-green-900/30 text-green-400 hover:bg-green-900/50"
                    : "bg-gray-800 text-gray-400 hover:bg-gray-700"
                }`}
              >
                {selected.enabled ? "Enabled" : "Disabled"}
              </button>
              <button
                onClick={() => {
                  onDeleteFlow(selected.id);
                  setSelectedIdx(0);
                }}
                className="px-3 py-1 text-sm rounded bg-red-900/30 text-red-400 hover:bg-red-900/50"
              >
                Delete
              </button>
            </div>
            <div className="text-xs text-gray-500">
              Double-click a flow in the list to open the editor.
            </div>
          </div>
        ) : (
          <div className="text-sm text-gray-500">Select a flow to view details, or create a new one.</div>
        )}
      </div>
    </div>
  );
}
