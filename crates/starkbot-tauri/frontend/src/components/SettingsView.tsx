import { useState } from "react";
import type { AppSnapshot } from "../types";

const MODEL_DESCRIPTIONS: Record<string, string> = {
  "gpt-5.4": "Default model. Good balance of speed and capability.",
  "gpt-5.4-mini": "Faster and cheaper. Best for simple tasks.",
  "gpt-5.5": "Most capable. Best for complex reasoning.",
};

const SECTIONS = ["Inference"] as const;
type Section = (typeof SECTIONS)[number];

export default function SettingsView({
  snapshot,
  currentModel,
  onSwitchModel,
  onAddApiKey,
}: {
  snapshot: AppSnapshot | null;
  currentModel: string;
  onSwitchModel: (model: string) => Promise<void>;
  onAddApiKey: (name: string, key: string) => Promise<void>;
}) {
  const [section, setSection] = useState<Section>("Inference");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [saving, setSaving] = useState(false);
  const models = snapshot?.available_models || [];
  const inferenceConfigured = snapshot?.inference_configured ?? false;

  // Check if OPENAI_API_KEY exists in api_keys
  const existingKey = snapshot?.api_keys?.find((k) => k.name === "OPENAI_API_KEY");

  const handleSaveKey = async () => {
    const key = apiKeyInput.trim();
    if (!key) return;
    setSaving(true);
    try {
      await onAddApiKey("OPENAI_API_KEY", key);
      setApiKeyInput("");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="h-full flex">
      {/* Section pills */}
      <div className="w-40 border-r border-surface-3 p-3 flex flex-col gap-1">
        {SECTIONS.map((s) => (
          <button
            key={s}
            onClick={() => setSection(s)}
            className={`px-3 py-1.5 text-sm rounded-full text-left transition-colors ${
              s === section
                ? "bg-accent/20 text-accent"
                : "text-gray-400 hover:bg-surface-2"
            }`}
          >
            {s}
          </button>
        ))}
      </div>

      {/* Section content */}
      <div className="flex-1 overflow-y-auto p-6">
        {section === "Inference" && (
          <>
            {/* Provider */}
            <h3 className="text-xs text-gray-500 uppercase mb-2">Provider</h3>
            <p className="text-sm text-white mb-6">OpenAI</p>

            {/* API Key */}
            <h3 className="text-xs text-gray-500 uppercase mb-3">API Key</h3>
            {existingKey ? (
              <div className="mb-6">
                <div className="flex items-center gap-3 mb-2">
                  <span className="text-sm text-green-400 font-mono">{existingKey.masked_key}</span>
                  <span className="text-xs text-green-600">Configured</span>
                </div>
                <div className="flex gap-2 items-center">
                  <input
                    type="password"
                    value={apiKeyInput}
                    onChange={(e) => setApiKeyInput(e.target.value)}
                    placeholder="Enter new key to replace..."
                    className="flex-1 max-w-md px-3 py-1.5 bg-surface-2 border border-surface-3 rounded text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-accent/50"
                  />
                  <button
                    onClick={handleSaveKey}
                    disabled={!apiKeyInput.trim() || saving}
                    className="px-4 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors disabled:opacity-30"
                  >
                    {saving ? "Saving..." : "Update"}
                  </button>
                </div>
              </div>
            ) : (
              <div className="mb-6">
                <p className="text-sm text-gray-400 mb-2">No API key configured. Add your OpenAI key to enable inference.</p>
                <div className="flex gap-2 items-center">
                  <input
                    type="password"
                    value={apiKeyInput}
                    onChange={(e) => setApiKeyInput(e.target.value)}
                    placeholder="sk-..."
                    className="flex-1 max-w-md px-3 py-1.5 bg-surface-2 border border-surface-3 rounded text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-accent/50"
                  />
                  <button
                    onClick={handleSaveKey}
                    disabled={!apiKeyInput.trim() || saving}
                    className="px-4 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors disabled:opacity-30"
                  >
                    {saving ? "Saving..." : "Save"}
                  </button>
                </div>
              </div>
            )}

            {/* Model selection */}
            <h3 className="text-xs text-gray-500 uppercase mb-2">Model</h3>
            <h2 className="text-lg font-semibold text-white mb-1">{currentModel}</h2>
            <p className="text-sm text-gray-400 mb-4">
              {MODEL_DESCRIPTIONS[currentModel] || ""}
            </p>

            <div className="flex flex-wrap gap-2">
              {models.map((model) => {
                const isCurrent = model === currentModel;
                return (
                  <button
                    key={model}
                    onClick={() => !isCurrent && onSwitchModel(model)}
                    disabled={!inferenceConfigured}
                    className={`px-4 py-2 rounded text-sm transition-colors ${
                      isCurrent
                        ? "bg-green-900/30 text-green-400 border border-green-700/30"
                        : inferenceConfigured
                          ? "text-gray-400 hover:bg-surface-2 border border-surface-3"
                          : "text-gray-600 border border-surface-3 opacity-50 cursor-not-allowed"
                    }`}
                  >
                    {model}
                    {isCurrent && <span className="ml-2 text-xs opacity-60">(active)</span>}
                  </button>
                );
              })}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
