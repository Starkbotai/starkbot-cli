import { useState } from "react";
import type { AppSnapshot, IntegrationPresetInfo } from "../types";

export default function ApiKeysView({
  snapshot,
  onAdd,
  onDelete,
  onInstall,
  onUninstall,
  onImportFlow,
}: {
  snapshot: AppSnapshot | null;
  onAdd: (name: string, key: string) => Promise<void>;
  onDelete: (name: string) => Promise<void>;
  onInstall: (presetId: string, apiKeys: [string, string][]) => Promise<void>;
  onUninstall: (presetId: string) => Promise<void>;
  onImportFlow: (presetId: string) => Promise<void>;
}) {
  const integrations = snapshot?.integrations ?? [];
  const keys = snapshot?.api_keys ?? [];
  const installed = integrations.filter((i) => i.installed);
  const available = integrations.filter((i) => !i.installed);

  const [adding, setAdding] = useState(false);
  const [selectedPreset, setSelectedPreset] = useState<IntegrationPresetInfo | null>(null);
  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});
  const [showKeys, setShowKeys] = useState(false);
  const [showManualKey, setShowManualKey] = useState(false);
  const [manualKeyName, setManualKeyName] = useState("");
  const [manualKeyValue, setManualKeyValue] = useState("");

  const handleInstall = async () => {
    if (!selectedPreset) return;
    const apiKeys: [string, string][] = [];
    if (selectedPreset.required_keys.length > 0) {
      for (const rk of selectedPreset.required_keys) {
        const val = (keyInputs[rk.name] ?? "").trim();
        if (val) apiKeys.push([rk.name, val]);
      }
    } else if (selectedPreset.api_key_name) {
      const val = (keyInputs[selectedPreset.api_key_name] ?? "").trim();
      if (val) apiKeys.push([selectedPreset.api_key_name, val]);
    }
    await onInstall(selectedPreset.id, apiKeys);
    setSelectedPreset(null);
    setKeyInputs({});
    setAdding(false);
  };

  const handleManualKeyAdd = async () => {
    if (!manualKeyName.trim() || !manualKeyValue.trim()) return;
    await onAdd(manualKeyName.trim().toUpperCase(), manualKeyValue.trim());
    setManualKeyName("");
    setManualKeyValue("");
    setShowManualKey(false);
  };

  const installDisabled = (() => {
    if (!selectedPreset) return true;
    if (selectedPreset.required_keys.length > 0) {
      return selectedPreset.required_keys.some((rk) => !(keyInputs[rk.name] ?? "").trim());
    }
    if (selectedPreset.api_key_name) {
      return !(keyInputs[selectedPreset.api_key_name] ?? "").trim();
    }
    return false;
  })();

  return (
    <div className="h-full flex flex-col p-6 overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-5">
        <h2 className="text-lg font-semibold text-white">Integrations</h2>
        {available.length > 0 && (
          <button
            onClick={() => { setAdding(!adding); setSelectedPreset(null); setKeyInputs({}); }}
            className="px-3 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors"
          >
            {adding ? "Cancel" : "+ Add Integration"}
          </button>
        )}
      </div>

      {/* Add integration flow */}
      {adding && (
        <div className="mb-5 p-4 rounded-lg border border-accent/30 bg-surface-1">
          <label className="block text-xs text-gray-500 mb-2">Select a preset to install</label>
          <div className="space-y-2 mb-3">
            {available.map((preset) => (
              <button
                key={preset.id}
                onClick={() => { setSelectedPreset(preset); setKeyInputs({}); }}
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

          {/* Multi-key inputs */}
          {selectedPreset && selectedPreset.required_keys.length > 0 && (
            <div className="space-y-3 mb-3">
              {selectedPreset.required_keys.map((rk) => {
                const isSecret = /KEY|TOKEN|SECRET|PASSWORD/i.test(rk.name);
                return (
                  <div key={rk.name}>
                    <label className="block text-xs text-gray-500 mb-1">
                      {rk.label} <span className="text-gray-600">({rk.name})</span>
                    </label>
                    <input
                      type={isSecret ? "password" : "text"}
                      value={keyInputs[rk.name] ?? ""}
                      onChange={(e) => setKeyInputs((prev) => ({ ...prev, [rk.name]: e.target.value }))}
                      placeholder={`Enter ${rk.label}...`}
                      className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
                    />
                  </div>
                );
              })}
            </div>
          )}

          {/* Legacy single-key input */}
          {selectedPreset && selectedPreset.required_keys.length === 0 && selectedPreset.api_key_name && (
            <div className="mb-3">
              <label className="block text-xs text-gray-500 mb-1">
                {selectedPreset.api_key_name} <span className="text-gray-600">(required)</span>
              </label>
              <input
                type="password"
                value={keyInputs[selectedPreset.api_key_name] ?? ""}
                onChange={(e) => setKeyInputs((prev) => ({ ...prev, [selectedPreset.api_key_name!]: e.target.value }))}
                placeholder="Enter API key..."
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
          )}

          {selectedPreset && (
            <button
              onClick={handleInstall}
              disabled={installDisabled}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors disabled:opacity-30"
            >
              Install {selectedPreset.name}
            </button>
          )}
        </div>
      )}

      {/* Installed integrations — primary content */}
      {installed.length > 0 ? (
        <div className="space-y-3 mb-6">
          {installed.map((integration) => (
            <div
              key={integration.id}
              className="px-5 py-4 rounded-lg border border-surface-3 bg-surface-1"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                  <span className="text-sm text-white font-semibold">{integration.name}</span>
                  {integration.configured ? (
                    <span className="px-1.5 py-0.5 text-[10px] rounded bg-green-900/40 text-green-400 border border-green-800/50">
                      Active
                    </span>
                  ) : (
                    <span className="px-1.5 py-0.5 text-[10px] rounded bg-yellow-900/40 text-yellow-400 border border-yellow-800/50">
                      Needs Config
                    </span>
                  )}
                </div>
                <div className="flex gap-2">
                  {integration.has_flow_template && (
                    <button
                      onClick={() => onImportFlow(integration.id)}
                      className="px-3 py-1 text-xs text-cyan-400 hover:text-cyan-300 hover:bg-cyan-900/20 rounded transition-colors"
                    >
                      Import Flow
                    </button>
                  )}
                  <button
                    onClick={() => onUninstall(integration.id)}
                    className="px-3 py-1 text-xs text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                  >
                    Uninstall
                  </button>
                </div>
              </div>
              <div className="text-xs text-gray-400 mt-1">{integration.description}</div>
              {integration.skills.length > 0 && (
                <div className="flex gap-1.5 mt-2">
                  {integration.skills.map((s) => (
                    <span key={s} className="px-1.5 py-0.5 text-[10px] rounded bg-surface-2 text-gray-400 border border-surface-3">
                      {s}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      ) : (
        !adding && (
          <div className="text-gray-500 text-sm mb-6">
            No integrations installed. Click "+ Add Integration" to get started.
          </div>
        )
      )}

      {/* API Keys — collapsible secondary section */}
      <div className="border-t border-surface-3 pt-4">
        <button
          onClick={() => setShowKeys(!showKeys)}
          className="flex items-center gap-2 text-xs text-gray-500 hover:text-gray-300 transition-colors mb-2"
        >
          <span>{showKeys ? "▾" : "▸"}</span>
          <span className="uppercase tracking-wider font-medium">API Keys</span>
          <span className="text-gray-600">({keys.length})</span>
        </button>

        {showKeys && (
          <div className="mt-2">
            <div className="flex justify-end mb-2">
              <button
                onClick={() => setShowManualKey(!showManualKey)}
                className="px-2 py-1 text-xs text-gray-400 hover:text-gray-200 hover:bg-surface-2 rounded transition-colors"
              >
                {showManualKey ? "Cancel" : "+ Add Key"}
              </button>
            </div>

            {/* Manual key form */}
            {showManualKey && (
              <div className="mb-3 p-3 rounded-lg border border-surface-3 bg-surface-1">
                <div className="space-y-2">
                  <div>
                    <label className="block text-xs text-gray-500 mb-1">Key Name</label>
                    <input
                      type="text"
                      value={manualKeyName}
                      onChange={(e) => setManualKeyName(e.target.value.toUpperCase())}
                      placeholder="OPENAI_API_KEY"
                      className="w-full px-3 py-1.5 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
                    />
                  </div>
                  <div>
                    <label className="block text-xs text-gray-500 mb-1">Value</label>
                    <input
                      type="password"
                      value={manualKeyValue}
                      onChange={(e) => setManualKeyValue(e.target.value)}
                      placeholder="sk-..."
                      className="w-full px-3 py-1.5 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
                    />
                  </div>
                  <button
                    onClick={handleManualKeyAdd}
                    disabled={!manualKeyName.trim() || !manualKeyValue.trim()}
                    className="px-3 py-1.5 bg-green-600 hover:bg-green-500 text-white text-xs rounded transition-colors disabled:opacity-30"
                  >
                    Save
                  </button>
                </div>
              </div>
            )}

            {keys.length > 0 ? (
              <div className="space-y-1">
                {keys.map((k) => (
                  <div
                    key={k.name}
                    className="flex items-center justify-between px-3 py-2 rounded border border-surface-3 bg-surface-1"
                  >
                    <div className="flex items-center gap-3">
                      <span className="text-xs text-white font-medium">{k.name}</span>
                      <span className="text-[10px] text-gray-600 font-mono">{k.masked_key}</span>
                    </div>
                    <button
                      onClick={() => onDelete(k.name)}
                      className="px-2 py-0.5 text-[10px] text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                    >
                      Delete
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-xs text-gray-600">No API keys stored.</div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
