import { useState } from "react";
import type { AppSnapshot } from "../types";

export default function ApiKeysView({
  snapshot,
  onAdd,
  onDelete,
}: {
  snapshot: AppSnapshot | null;
  onAdd: (name: string, key: string) => Promise<void>;
  onDelete: (name: string) => Promise<void>;
}) {
  const keys = snapshot?.api_keys || [];
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  const [keyValue, setKeyValue] = useState("");

  const handleAdd = async () => {
    if (!name.trim() || !keyValue.trim()) return;
    await onAdd(name.trim().toUpperCase(), keyValue.trim());
    setName("");
    setKeyValue("");
    setAdding(false);
  };

  return (
    <div className="h-full flex flex-col p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-white">API Keys</h2>
        <button
          onClick={() => setAdding(!adding)}
          className="px-3 py-1.5 bg-accent hover:bg-accent-dim text-white text-sm rounded transition-colors"
        >
          {adding ? "Cancel" : "Add Key"}
        </button>
      </div>

      {/* Add form */}
      {adding && (
        <div className="mb-4 p-4 rounded-lg border border-surface-3 bg-surface-1">
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Service Name (UPPER_SNAKE_CASE)</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value.toUpperCase())}
                placeholder="OPENAI_API_KEY"
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">API Key</label>
              <input
                type="password"
                value={keyValue}
                onChange={(e) => setKeyValue(e.target.value)}
                placeholder="sk-..."
                className="w-full px-3 py-2 bg-surface-2 border border-surface-3 rounded text-sm text-white placeholder-gray-600 focus:outline-none focus:border-accent/50"
              />
            </div>
            <button
              onClick={handleAdd}
              disabled={!name.trim() || !keyValue.trim()}
              className="px-4 py-2 bg-green-600 hover:bg-green-500 text-white text-sm rounded transition-colors disabled:opacity-30"
            >
              Save
            </button>
          </div>
        </div>
      )}

      {/* Key list */}
      {keys.length === 0 ? (
        <div className="text-gray-500 text-sm">No API keys configured. Click "Add Key" to add one.</div>
      ) : (
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
      )}
    </div>
  );
}
