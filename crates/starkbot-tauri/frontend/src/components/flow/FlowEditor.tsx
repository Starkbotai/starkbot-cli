import { useCallback, useEffect, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  addEdge,
  useNodesState,
  useEdgesState,
  type Connection,
  type Node,
  type Edge,
  type NodeTypes,
  ReactFlowProvider,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import type { SavedFlow, FlowDefinition } from "../../types";
import EntryNode from "./EntryNode";
import PromptNode from "./PromptNode";
import BranchNode from "./BranchNode";
import ToolBranchNode from "./ToolBranchNode";

const nodeTypes: NodeTypes = {
  entry: EntryNode,
  prompt: PromptNode,
  branch: BranchNode,
  branch_tool: ToolBranchNode,
};

const defaultNodeData: Record<string, Record<string, unknown>> = {
  entry: { schedule_type: "minutes", interval: 5 },
  prompt: { prompt: "" },
  branch: { condition: "" },
  branch_tool: { condition: "", outputs: [] },
};

let nodeIdCounter = 0;

function ToolBranchConfig({
  nodeId,
  data,
  onDataChange,
}: {
  nodeId: string;
  data: Record<string, unknown>;
  onDataChange: (nodeId: string, key: string, value: unknown) => void;
}) {
  const condition = (data.condition as string) || "";
  const outputs = (data.outputs as string[]) || [];
  const [newOutput, setNewOutput] = useState("");

  const addOutput = () => {
    const name = newOutput.trim().toLowerCase().replace(/\s+/g, "_");
    if (!name || outputs.includes(name)) return;
    onDataChange(nodeId, "outputs", [...outputs, name]);
    setNewOutput("");
  };

  const removeOutput = (name: string) => {
    onDataChange(nodeId, "outputs", outputs.filter((o) => o !== name));
  };

  return (
    <div className="space-y-3">
      <div>
        <label className="block text-xs text-gray-400 mb-1">Condition / instruction</label>
        <textarea
          value={condition}
          onChange={(e) => onDataChange(nodeId, "condition", e.target.value)}
          rows={4}
          className="w-full px-2 py-1.5 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none resize-none"
          placeholder="Classify the user's intent..."
        />
        <p className="text-[10px] text-gray-500 mt-1">
          The LLM receives these outputs as tool call options and picks one.
        </p>
      </div>

      <div>
        <label className="block text-xs text-gray-400 mb-1">Outputs (tool options)</label>
        <div className="space-y-1 mb-2">
          {outputs.map((output) => (
            <div
              key={output}
              className="flex items-center justify-between px-2 py-1 rounded bg-surface-0 border border-surface-3"
            >
              <span className="text-xs text-gray-200 font-mono">{output}</span>
              <button
                onClick={() => removeOutput(output)}
                className="text-[10px] text-red-400 hover:text-red-300"
              >
                x
              </button>
            </div>
          ))}
        </div>
        <div className="flex gap-1">
          <input
            type="text"
            value={newOutput}
            onChange={(e) => setNewOutput(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") addOutput(); }}
            placeholder="output_name"
            className="flex-1 px-2 py-1 text-xs rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none"
          />
          <button
            onClick={addOutput}
            disabled={!newOutput.trim()}
            className="px-2 py-1 text-xs rounded bg-cyan-600 text-white hover:bg-cyan-500 disabled:opacity-30"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}

interface FlowEditorProps {
  flow: SavedFlow;
  onSave: (flow: SavedFlow) => void;
  onClose: () => void;
}

function FlowEditorInner({ flow, onSave, onClose }: FlowEditorProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([] as Node[]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([] as Edge[]);
  const [flowName, setFlowName] = useState(flow.name);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [reactFlowInstance, setReactFlowInstance] = useState<any>(null);

  // Load flow definition into ReactFlow state
  useEffect(() => {
    const def = flow.flow;
    if (def.nodes.length > 0) {
      setNodes(
        def.nodes.map((n) => ({
          id: n.id,
          type: n.node_type,
          position: { x: n.position[0], y: n.position[1] },
          data: n.data,
        }))
      );
      setEdges(
        def.edges.map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          sourceHandle: e.source_handle ?? undefined,
          targetHandle: e.target_handle ?? undefined,
          animated: true,
          style: { stroke: "#818cf8" },
        }))
      );
      nodeIdCounter =
        Math.max(
          0,
          ...def.nodes.map((n) => {
            const num = parseInt(n.id.replace(/\D/g, ""));
            return isNaN(num) ? 0 : num;
          })
        ) + 1;
    } else {
      // New flow — auto-create entry node
      nodeIdCounter = 1;
      setNodes([
        {
          id: "node_0",
          type: "entry",
          position: { x: 250, y: 50 },
          data: { ...defaultNodeData.entry },
        },
      ]);
      setHasUnsavedChanges(true);
    }
  }, [flow.id]);

  const hasEntryNode = nodes.some((n) => n.type === "entry");

  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => {
        const filtered = eds.filter(
          (e) => !(e.source === connection.source && e.sourceHandle === connection.sourceHandle)
        );
        return addEdge(
          { ...connection, animated: true, style: { stroke: "#818cf8" } },
          filtered
        );
      });
      setHasUnsavedChanges(true);
    },
    [setEdges]
  );

  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNode(null);
  }, []);

  const onNodesChangeWrapped = useCallback(
    (changes: any) => {
      // Prevent deletion of entry node
      const filtered = changes.filter((c: any) => {
        if (c.type === "remove") {
          const node = nodes.find((n: Node) => n.id === c.id);
          if (node?.type === "entry") return false;
        }
        return true;
      });
      onNodesChange(filtered);
      if (filtered.some((c: any) => c.type !== "select")) {
        setHasUnsavedChanges(true);
      }
    },
    [onNodesChange, nodes]
  );

  const onEdgesChangeWrapped = useCallback(
    (changes: any) => {
      onEdgesChange(changes);
      setHasUnsavedChanges(true);
    },
    [onEdgesChange]
  );

  const addNode = useCallback(
    (type: "prompt" | "branch" | "branch_tool") => {
      const position = { x: 250, y: 100 + nodes.length * 150 };
      const newNode: Node = {
        id: `node_${nodeIdCounter++}`,
        type,
        position,
        data: { ...defaultNodeData[type] },
      };
      setNodes((nds) => [...nds, newNode]);
      setHasUnsavedChanges(true);
    },
    [nodes.length, setNodes]
  );

  // Drag and drop
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const type = event.dataTransfer.getData("application/reactflow") as "prompt" | "branch" | "branch_tool";
      if (!type || !reactFlowInstance) return;
      const position = reactFlowInstance.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      const newNode: Node = {
        id: `node_${nodeIdCounter++}`,
        type,
        position,
        data: { ...defaultNodeData[type] },
      };
      setNodes((nds) => [...nds, newNode]);
      setHasUnsavedChanges(true);
    },
    [reactFlowInstance, setNodes]
  );

  const handleSave = useCallback(() => {
    const definition: FlowDefinition = {
      nodes: nodes.map((n) => ({
        id: n.id,
        node_type: (n.type as "entry" | "prompt" | "branch" | "branch_tool") || "prompt",
        data: n.data as Record<string, unknown>,
        position: [n.position.x, n.position.y] as [number, number],
      })),
      edges: edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        source_handle: e.sourceHandle ?? undefined,
        target_handle: e.targetHandle ?? undefined,
      })),
    };
    const now = new Date().toISOString();
    onSave({
      ...flow,
      name: flowName,
      flow: definition,
      updated_at: now,
    });
    setHasUnsavedChanges(false);
  }, [nodes, edges, flowName, flow, onSave]);

  // Update node data when editing in the config panel
  const onNodeDataChange = useCallback(
    (nodeId: string, key: string, value: unknown) => {
      setNodes((nds) =>
        nds.map((n) => (n.id === nodeId ? { ...n, data: { ...n.data, [key]: value } } : n))
      );
      setSelectedNode((prev) =>
        prev?.id === nodeId ? { ...prev, data: { ...prev.data, [key]: value } } : prev
      );
      setHasUnsavedChanges(true);
    },
    [setNodes]
  );

  const deleteNode = useCallback(
    (nodeId: string) => {
      // Don't allow deleting entry node
      const node = nodes.find((n) => n.id === nodeId);
      if (node?.type === "entry") return;
      setNodes((nds) => nds.filter((n) => n.id !== nodeId));
      setEdges((eds) => eds.filter((e) => e.source !== nodeId && e.target !== nodeId));
      setSelectedNode((prev) => (prev?.id === nodeId ? null : prev));
      setHasUnsavedChanges(true);
    },
    [nodes, setNodes, setEdges]
  );

  // Ctrl+S to save, Escape to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleSave, onClose]);

  const onDragStartPalette = (event: React.DragEvent, nodeType: string) => {
    event.dataTransfer.setData("application/reactflow", nodeType);
    event.dataTransfer.effectAllowed = "move";
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-surface-0">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-4 py-2 bg-surface-1 border-b border-surface-3">
        <button
          onClick={onClose}
          className="px-2 py-1 text-sm rounded text-gray-400 hover:text-gray-200 hover:bg-surface-2"
        >
          Close
        </button>
        <input
          value={flowName}
          onChange={(e) => {
            setFlowName(e.target.value);
            setHasUnsavedChanges(true);
          }}
          className="px-2 py-1 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none w-48"
          placeholder="Flow name"
        />
        {hasUnsavedChanges && <span className="w-2 h-2 rounded-full bg-amber-400" title="Unsaved changes" />}
        <div className="flex-1" />

        {/* Draggable palette items */}
        <div
          className="px-3 py-1 text-xs rounded bg-indigo-500/20 text-indigo-300 cursor-grab hover:bg-indigo-500/30"
          draggable
          onDragStart={(e) => onDragStartPalette(e, "prompt")}
        >
          + Prompt
        </div>
        <div
          className="px-3 py-1 text-xs rounded bg-amber-500/20 text-amber-300 cursor-grab hover:bg-amber-500/30"
          draggable
          onDragStart={(e) => onDragStartPalette(e, "branch")}
        >
          + Branch
        </div>
        <div
          className="px-3 py-1 text-xs rounded bg-cyan-500/20 text-cyan-300 cursor-grab hover:bg-cyan-500/30"
          draggable
          onDragStart={(e) => onDragStartPalette(e, "branch_tool")}
        >
          + Tool Branch
        </div>

        <button
          onClick={handleSave}
          className="px-3 py-1 text-sm rounded bg-accent text-white hover:bg-accent/80"
        >
          Save
        </button>
      </div>

      {/* Canvas + config panel */}
      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 relative">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChangeWrapped}
            onEdgesChange={onEdgesChangeWrapped}
            onConnect={onConnect}
            onNodeClick={onNodeClick}
            onPaneClick={onPaneClick}
            onDragOver={onDragOver}
            onDrop={onDrop}
            onInit={setReactFlowInstance}
            nodeTypes={nodeTypes}
            fitView
            deleteKeyCode="Delete"
            colorMode="dark"
            defaultEdgeOptions={{ animated: true, style: { stroke: "#818cf8" } }}
          >
            <Background gap={20} size={1} />
            <Controls />
          </ReactFlow>

          {/* Overlay if no entry node (shouldn't happen, but safety) */}
          {!hasEntryNode && (
            <div className="absolute inset-0 flex items-center justify-center bg-black/40 z-10">
              <div className="bg-surface-1 rounded-xl p-6 shadow-xl text-center max-w-sm border border-surface-3">
                <h3 className="text-lg font-semibold text-gray-200 mb-2">Entry node missing</h3>
                <p className="text-sm text-gray-400 mb-4">Every flow needs an entry node to define when it runs.</p>
                <button
                  onClick={() => {
                    setNodes((nds) => [
                      ...nds,
                      {
                        id: `node_${nodeIdCounter++}`,
                        type: "entry",
                        position: { x: 250, y: 50 },
                        data: { ...defaultNodeData.entry },
                      },
                    ]);
                    setHasUnsavedChanges(true);
                  }}
                  className="px-4 py-2 rounded bg-green-600 text-white hover:bg-green-500"
                >
                  Add Entry Node
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Config panel */}
        {selectedNode && (
          <div className="w-72 border-l border-surface-3 bg-surface-1 overflow-y-auto p-3">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-gray-200 capitalize">
                {selectedNode.type === "entry" ? "Entry (Timed)" : `${selectedNode.type} Node`}
              </h3>
              {selectedNode.type !== "entry" && (
                <button
                  onClick={() => deleteNode(selectedNode.id)}
                  className="text-xs text-red-400 hover:text-red-300"
                >
                  Delete
                </button>
              )}
            </div>
            <div className="text-xs text-gray-500 mb-3">ID: {selectedNode.id}</div>

            {selectedNode.type === "entry" && (
              <div className="space-y-3">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Schedule type</label>
                  <select
                    value={(selectedNode.data as Record<string, unknown>).schedule_type as string || "minutes"}
                    onChange={(e) => onNodeDataChange(selectedNode.id, "schedule_type", e.target.value)}
                    className="w-full px-2 py-1.5 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none appearance-none cursor-pointer"
                    style={{ colorScheme: "dark" }}
                  >
                    <option value="minutes">Minutes</option>
                    <option value="hours">Hours</option>
                  </select>
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Interval</label>
                  <input
                    type="number"
                    min="1"
                    value={(selectedNode.data as Record<string, unknown>).interval as number || 5}
                    onChange={(e) => onNodeDataChange(selectedNode.id, "interval", parseInt(e.target.value) || 1)}
                    className="w-full px-2 py-1.5 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none"
                  />
                </div>
                <p className="text-[10px] text-gray-500">
                  This flow will activate{" "}
                  {(selectedNode.data as Record<string, unknown>).schedule_type === "hours"
                    ? `every ${(selectedNode.data as Record<string, unknown>).interval || 5} hour(s)`
                    : `every ${(selectedNode.data as Record<string, unknown>).interval || 5} minute(s)`}.
                </p>
              </div>
            )}

            {selectedNode.type === "prompt" && (
              <div>
                <label className="block text-xs text-gray-400 mb-1">Prompt text</label>
                <textarea
                  value={(selectedNode.data as Record<string, unknown>).prompt as string || ""}
                  onChange={(e) => onNodeDataChange(selectedNode.id, "prompt", e.target.value)}
                  rows={6}
                  className="w-full px-2 py-1.5 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none resize-none"
                  placeholder="Enter prompt text..."
                />
              </div>
            )}

            {selectedNode.type === "branch" && (
              <div>
                <label className="block text-xs text-gray-400 mb-1">Condition</label>
                <textarea
                  value={(selectedNode.data as Record<string, unknown>).condition as string || ""}
                  onChange={(e) => onNodeDataChange(selectedNode.id, "condition", e.target.value)}
                  rows={4}
                  className="w-full px-2 py-1.5 text-sm rounded bg-surface-0 border border-surface-3 text-gray-200 focus:border-accent outline-none resize-none"
                  placeholder="Enter condition..."
                />
                <p className="text-[10px] text-gray-500 mt-1">
                  True output on left, False output on right.
                </p>
              </div>
            )}

            {selectedNode.type === "branch_tool" && (
              <ToolBranchConfig
                nodeId={selectedNode.id}
                data={selectedNode.data as Record<string, unknown>}
                onDataChange={onNodeDataChange}
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default function FlowEditor(props: FlowEditorProps) {
  return (
    <ReactFlowProvider>
      <FlowEditorInner {...props} />
    </ReactFlowProvider>
  );
}
