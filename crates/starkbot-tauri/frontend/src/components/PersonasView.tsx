import { useState } from "react";
import type { AppSnapshot } from "../types";

export default function PersonasView({ snapshot }: { snapshot: AppSnapshot | null }) {
  const [selected, setSelected] = useState(0);
  const personas = snapshot?.personas || [];

  if (!personas.length) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        No personas loaded
      </div>
    );
  }

  const persona = personas[selected];

  return (
    <div className="h-full flex">
      {/* List */}
      <div className="w-64 border-r border-surface-3 overflow-y-auto">
        {personas.map((p, i) => (
          <button
            key={p.key}
            onClick={() => setSelected(i)}
            className={`w-full text-left px-4 py-2 text-sm transition-colors ${
              i === selected
                ? "bg-accent/20 text-accent-light border-l-2 border-accent"
                : "text-gray-400 hover:bg-surface-2 border-l-2 border-transparent"
            }`}
          >
            {p.emoji} {p.label}
          </button>
        ))}
      </div>

      {/* Detail */}
      <div className="flex-1 overflow-y-auto p-6">
        <h2 className="text-lg font-semibold text-white mb-1">
          {persona.emoji} {persona.label}
        </h2>
        <p className="text-sm text-gray-400 mb-4">{persona.description}</p>

        <div className="space-y-2 text-sm mb-4">
          <div>
            <span className="text-gray-500">Status: </span>
            <span className={persona.enabled ? "text-green-400" : "text-red-400"}>
              {persona.enabled ? "Enabled" : "Disabled"}
            </span>
          </div>
          {persona.tool_groups.length > 0 && (
            <div>
              <span className="text-gray-500">Tool groups: </span>
              <span className="text-gray-300">{persona.tool_groups.join(", ")}</span>
            </div>
          )}
          {persona.skill_tags.length > 0 && (
            <div>
              <span className="text-gray-500">Skill tags: </span>
              <span className="text-gray-300">{persona.skill_tags.join(", ")}</span>
            </div>
          )}
        </div>

        <div className="border-t border-surface-3 pt-4">
          <h3 className="text-xs text-gray-500 uppercase mb-2">System Prompt Preview</h3>
          <pre className="text-xs text-gray-300 whitespace-pre-wrap font-mono">
            {persona.system_prompt_preview}
          </pre>
        </div>
      </div>
    </div>
  );
}
