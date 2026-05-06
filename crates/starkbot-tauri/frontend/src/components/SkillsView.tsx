import { useState } from "react";
import type { AppSnapshot } from "../types";

export default function SkillsView({ snapshot }: { snapshot: AppSnapshot | null }) {
  const [selected, setSelected] = useState(0);
  const skills = snapshot?.skills || [];
  const skillsDir = snapshot?.skills_dir || "";

  if (!skills.length) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        No skills loaded
      </div>
    );
  }

  const skill = skills[selected];

  return (
    <div className="h-full flex">
      {/* List */}
      <div className="w-64 border-r border-surface-3 overflow-y-auto">
        {skills.map((s, i) => (
          <button
            key={s.name}
            onClick={() => setSelected(i)}
            className={`w-full text-left px-4 py-2 text-sm transition-colors ${
              i === selected
                ? "bg-accent/20 text-accent-light border-l-2 border-accent"
                : "text-gray-400 hover:bg-surface-2 border-l-2 border-transparent"
            }`}
          >
            {s.name}
          </button>
        ))}
      </div>

      {/* Detail */}
      <div className="flex-1 overflow-y-auto p-6">
        <h2 className="text-lg font-semibold text-white mb-2">{skill.name}</h2>
        <p className="text-sm text-gray-400 mb-4">{skill.description}</p>

        <div className="flex gap-2 mb-4">
          {skill.tags.map((tag) => (
            <span key={tag} className="px-2 py-0.5 bg-surface-2 text-xs text-gray-400 rounded">
              {tag}
            </span>
          ))}
        </div>

        {skill.requires_tools.length > 0 && (
          <div className="mb-4">
            <span className="text-xs text-gray-500">Required tools: </span>
            <span className="text-xs text-yellow-400">{skill.requires_tools.join(", ")}</span>
          </div>
        )}

        <div className="border-t border-surface-3 pt-4">
          <pre className="text-xs text-gray-300 whitespace-pre-wrap font-mono">{skill.content}</pre>
        </div>
      </div>
    </div>
  );
}
