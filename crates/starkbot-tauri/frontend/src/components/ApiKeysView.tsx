import { useState } from "react";
import type { AppSnapshot, IntegrationPresetInfo } from "../types";

export default function ApiKeysView({
  snapshot,
  onAdd,
  onDelete,
  onInstall,
  onUninstall,
}: {
  snapshot: AppSnapshot | null;
  onAdd: (name: string, key: string) => Promise<void>;
  onDelete: (name: string) => Promise<void>;
  onInstall: (presetId: string, apiKey?: string) => Promise<void>;
  onUninstall: (presetId: string) => Promise<void>;
}) {
  const integrations = snapshot?.integrations ?? [];
  const keys = snapshot?.api_keys ?? [];
  const installed = integrations.filter((i) => i.installed);
  const available = integrations.filter((i) => !i.installed);

  const [adding, setAdding] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState<IntegrationPresetInfo | null>(null);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [showManualKey, setShowManualKey] = useState(false);
  const [manualKeyName, setManualKeyName] = useState("");
  const [manualKeyValue, setManualKeyValue] = useState("");

  const handleInstall = async () => {
    if (!selectedPreset) return;
    const key = selectedPreset.api_key_name && apiKeyInput.trim() ? apiKeyInput.trim() : undefined;
    await onInstall(selectedPreset.id, key);
    setSelectedPreset(null);
    setApiKeyInput("");
    setAdding(false);
  };

  const handleManualKeyAdd = async () => {
    if (!manualKeyName.trim() || !manualKeyValue.trim()) return;
    await onAdd(manualKeyName.trim().toUpperCase(), manualKeyValue.trim());
    setManualKeyName("");
    setManualKeyValue("");
    setShowManualKey(false);
  };

  return (
    <div className="h-full flex flex-col p-6 overflow-y-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-white">Integrations</h2>
        <div className="flex gap-2">
          <button
            onClick={() => setShowManualKey(!showManualKey)}
            className="px-3 py-1.5 text-gray-400 hover:text-white text-sm rounded border border-surface-3 hover:border-gray-500 transition-colors"
          >
            {showManualKey ? "Cancel" : "Add Key"}
          </button>
          {available.length > 0 && (
            <button
              onClick={() => { setAdding(!adding); setSelectedPreset(null); setApiKeyInput(""); }}
              className="px-3 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors"
            >
              {adding ? "Cancel" : "Add Integration"}
            </button>
          )}
        </div>
      </div>

      {/* Manual key form */}
      {showManualKey && (
        <div className="mb-4 p-4 rounded-lg border border-surface-3 bg-surface-1">
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Key Name (UPPER_SNAKE_CASE)</label>
              <input
                type="text"
                value={manualKeyName}
                onChange={(e) => setManualKeyName(e.target.value.toUpperCase())}
                placeholder="OPENAI_API_KEY"
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">API Key</label>
              <input
                type="password"
                value={manualKeyValue}
                onChange={(e) => setManualKeyValue(e.target.value)}
                placeholder="sk-..."
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
            <button
              onClick={handleManualKeyAdd}
              disabled={!manualKeyName.trim() || !manualKeyValue.trim()}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors disabled:opacity-30"
            >
              Save
            </button>
          </div>
        </div>
      )}

      {/* Add integration flow */}
      {adding && (
        <div className="mb-4 p-4 rounded-lg border border-accent/30 bg-surface-1">
          <label className="block text-xs text-gray-500 mb-2">Select a preset to install</label>
          <div className="space-y-2 mb-3">
            {available.map((preset) => (
              <button
                key={preset.id}
                onClick={() => { setSelectedPreset(preset); setApiKeyInput(""); }}
                className={`w-full text-left px-4 py-3 rounded-lg border transition-colors ${
                  selectedPreset?.id === preset.id
                    ? "border-accent bg-accent/10"
                    : "border-surface-3 hover:border-gray-500 bg-surface-2"
                }`}
              >
                <div className="text-sm text-white font-medium">{preset.name}</div>
                <div className="text-xs text-gray-400 mt-0.5">{preset.description}</div>
              </button>
            ))}
          </div>

          {selectedPreset && selectedPreset.api_key_name && (
            <div className="mb-3">
              <label className="block text-xs text-gray-500 mb-1">
                {selectedPreset.api_key_name} <span className="text-gray-600">(required)</span>
              </label>
              <input
                type="password"
                value={apiKeyInput}
                onChange={(e) => setApiKeyInput(e.target.value)}
                placeholder="Enter API key..."
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
          )}

          {selectedPreset && (
            <button
              onClick={handleInstall}
              disabled={!!selectedPreset.api_key_name && !apiKeyInput.trim()}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors disabled:opacity-30"
            >
              Install {selectedPreset.name}
            </button>
          )}
        </div>
      )}

      {/* Installed integrations */}
      {installed.length > 0 && (
        <div className="mb-6">
          <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-2">Installed</h3>
          <div className="space-y-2">
            {installed.map((integration) => (
              <div
                key={integration.id}
                className="flex items-center justify-between px-4 py-3 rounded-lg border border-surface-3 bg-surface-1"
              >
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-white font-medium">{integration.name}</span>
                    {integration.configured ? (
                      <span className="px-1.5 py-0.5 text-[10px] rounded bg-green-900/40 text-green-400 border border-green-800/50">
                        Configured
                      </span>
                    ) : (
                      <span className="px-1.5 py-0.5 text-[10px] rounded bg-yellow-900/40 text-yellow-400 border border-yellow-800/50">
                        Needs API Key
                      </span>
                    )}
                  </div>
                  <div className="text-xs text-gray-400 mt-0.5">{integration.description}</div>
                  {integration.skills.length > 0 && (
                    <div className="text-[10px] text-gray-500 mt-1">
                      Skills: {integration.skills.join(", ")}
                    </div>
                  )}
                </div>
                <button
                  onClick={() => onUninstall(integration.id)}
                  className="px-3 py-1 text-xs text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors ml-3"
                >
                  Uninstall
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Raw API Keys */}
      {keys.length > 0 && (
        <div>
          <h3 className="text-xs text-gray-500 uppercase tracking-wider mb-2">API Keys</h3>
          <div className="space-y-2">
            {keys.map((k) => (
              <div
                key={k.name}
                className="flex items-center justify-between px-4 py-3 rounded-lg border border-surface-3 bg-surface-1"
              >
                <div>
                  <div className="text-sm text-white font-medium">{k.name}</div>
                  <div className="text-xs text-gray-500 font-mono">{k.masked_key}</div>
                </div>
                <button
                  onClick={() => onDelete(k.name)}
                  className="px-3 py-1 text-xs text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                >
                  Delete
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {installed.length === 0 && keys.length === 0 && !adding && !showManualKey && (
        <div className="text-gray-500 text-sm">
          No integrations installed. Click "Add Integration" to get started.
        </div>
      )}
    </div>
  );
}
