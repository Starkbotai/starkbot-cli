import type { AppSnapshot } from "../types";

const MODEL_DESCRIPTIONS: Record<string, string> = {
  "gpt-5.4": "Default model. Good balance of speed and capability.",
  "gpt-5.4-mini": "Faster and cheaper. Best for simple tasks.",
  "gpt-5.5": "Most capable. Best for complex reasoning.",
};

export default function SettingsView({
  snapshot,
  currentModel,
  onSwitchModel,
}: {
  snapshot: AppSnapshot | null;
  currentModel: string;
  onSwitchModel: (model: string) => Promise<void>;
}) {
  const models = snapshot?.available_models || [];

  return (
    <div className="h-full flex">
      {/* Model list */}
      <div className="w-64 border-r border-surface-3 overflow-y-auto p-4">
        <h3 className="text-xs text-gray-500 uppercase mb-3">Models</h3>
        {models.map((model) => {
          const isCurrent = model === currentModel;
          return (
            <button
              key={model}
              onClick={() => !isCurrent && onSwitchModel(model)}
              className={`w-full text-left px-3 py-2 mb-1 rounded text-sm transition-colors ${
                isCurrent
                  ? "bg-green-900/30 text-green-400 border border-green-700/30"
                  : "text-gray-400 hover:bg-surface-2"
              }`}
            >
              {model}
              {isCurrent && <span className="ml-2 text-xs opacity-60">(active)</span>}
            </button>
          );
        })}
      </div>

      {/* Info */}
      <div className="flex-1 p-6">
        <h2 className="text-lg font-semibold text-white mb-2">Model: {currentModel}</h2>
        <p className="text-sm text-gray-400">
          {MODEL_DESCRIPTIONS[currentModel] || ""}
        </p>
      </div>
    </div>
  );
}
