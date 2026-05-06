import { Handle, Position, type NodeProps } from "@xyflow/react";

export default function ToolBranchNode({ data, selected }: NodeProps) {
  const d = data as Record<string, unknown>;
  const condition = (d.condition as string) || "";
  const outputs = (d.outputs as string[]) || [];

  // Color palette for output handles
  const colors = [
    { bg: "!bg-cyan-500", border: "!border-cyan-700", text: "text-cyan-400" },
    { bg: "!bg-violet-500", border: "!border-violet-700", text: "text-violet-400" },
    { bg: "!bg-orange-500", border: "!border-orange-700", text: "text-orange-400" },
    { bg: "!bg-pink-500", border: "!border-pink-700", text: "text-pink-400" },
    { bg: "!bg-teal-500", border: "!border-teal-700", text: "text-teal-400" },
    { bg: "!bg-yellow-500", border: "!border-yellow-700", text: "text-yellow-400" },
    { bg: "!bg-lime-500", border: "!border-lime-700", text: "text-lime-400" },
    { bg: "!bg-rose-500", border: "!border-rose-700", text: "text-rose-400" },
  ];

  return (
    <div
      className={`rounded-lg border shadow-lg min-w-[200px] bg-surface-1 ${
        selected ? "border-accent ring-1 ring-accent/50" : "border-surface-3"
      }`}
    >
      <Handle
        type="target"
        position={Position.Top}
        className="!w-3 !h-3 !bg-gray-500 !border-gray-600"
      />
      <div className="flex items-center gap-2 px-3 py-2 border-b border-surface-3 rounded-t-lg bg-cyan-500/10">
        <svg
          className="w-4 h-4 text-cyan-400"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={1.5}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M11.42 15.17L17.25 21A2.652 2.652 0 0021 17.25l-5.877-5.877M11.42 15.17l2.496-3.03c.317-.384.74-.626 1.208-.766M11.42 15.17l-4.655 5.653a2.548 2.548 0 11-3.586-3.586l6.837-5.63m5.108-.233c.55-.164 1.163-.188 1.743-.14a4.5 4.5 0 004.486-6.336l-3.276 3.277a3.004 3.004 0 01-2.25-2.25l3.276-3.276a4.5 4.5 0 00-6.336 4.486c.091 1.076-.071 2.264-.904 2.95l-.102.085"
          />
        </svg>
        <span className="text-xs font-medium text-gray-200">Tool Branch</span>
      </div>
      <div className="px-3 py-2 text-xs text-gray-400 max-w-[260px]">
        {condition ? (
          <p className="truncate">{condition}</p>
        ) : (
          <p className="italic">No condition set</p>
        )}
      </div>

      {/* Output labels */}
      {outputs.length > 0 && (
        <div className="flex flex-wrap justify-around px-2 pb-1 pt-1 gap-1">
          {outputs.map((output, i) => {
            const color = colors[i % colors.length];
            return (
              <span key={output} className={`text-[9px] ${color.text}`}>
                {output}
              </span>
            );
          })}
        </div>
      )}

      {/* Dynamic output handles */}
      <div className="relative" style={{ height: 12 }}>
        {outputs.map((output, i) => {
          const color = colors[i % colors.length];
          const count = outputs.length;
          const pct = count === 1 ? 50 : (100 * (i + 1)) / (count + 1);
          return (
            <Handle
              key={output}
              type="source"
              position={Position.Bottom}
              id={`tool_${output}`}
              className={`!w-3 !h-3 ${color.bg} ${color.border}`}
              style={{ left: `${pct}%` }}
            />
          );
        })}
        {outputs.length === 0 && (
          <div className="text-center text-[9px] text-gray-600 italic">
            No outputs defined
          </div>
        )}
      </div>
    </div>
  );
}
