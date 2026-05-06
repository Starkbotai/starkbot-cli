import { Handle, Position, type NodeProps } from "@xyflow/react";

export default function BranchNode({ data, selected }: NodeProps) {
  const condition = (data as Record<string, unknown>).condition as string || "";
  return (
    <div className={`rounded-lg border shadow-lg min-w-[200px] bg-surface-1 ${selected ? "border-accent ring-1 ring-accent/50" : "border-surface-3"}`}>
      <Handle type="target" position={Position.Top} className="!w-3 !h-3 !bg-gray-500 !border-gray-600" />
      <div className="flex items-center gap-2 px-3 py-2 border-b border-surface-3 rounded-t-lg bg-amber-500/10">
        <svg className="w-4 h-4 text-amber-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
          <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 4.875c0-.621.504-1.125 1.125-1.125h4.5c.621 0 1.125.504 1.125 1.125v4.5c0 .621-.504 1.125-1.125 1.125h-4.5A1.125 1.125 0 013.75 9.375v-4.5zM3.75 14.625c0-.621.504-1.125 1.125-1.125h4.5c.621 0 1.125.504 1.125 1.125v4.5c0 .621-.504 1.125-1.125 1.125h-4.5a1.125 1.125 0 01-1.125-1.125v-4.5zM13.5 4.875c0-.621.504-1.125 1.125-1.125h4.5c.621 0 1.125.504 1.125 1.125v4.5c0 .621-.504 1.125-1.125 1.125h-4.5A1.125 1.125 0 0113.5 9.375v-4.5z" />
        </svg>
        <span className="text-xs font-medium text-gray-200">Branch</span>
      </div>
      <div className="px-3 py-2 text-xs text-gray-400 max-w-[220px]">
        {condition ? <p className="truncate">{condition}</p> : <p className="italic">No condition set</p>}
      </div>
      <div className="flex justify-around px-2 pb-1 pt-1">
        <span className="text-[9px] text-green-400">True</span>
        <span className="text-[9px] text-red-400">False</span>
      </div>
      <div className="relative" style={{ height: 12 }}>
        <Handle type="source" position={Position.Bottom} id="true" className="!w-3 !h-3 !bg-green-500 !border-green-700" style={{ left: "33%" }} />
        <Handle type="source" position={Position.Bottom} id="false" className="!w-3 !h-3 !bg-red-500 !border-red-700" style={{ left: "67%" }} />
      </div>
    </div>
  );
}
