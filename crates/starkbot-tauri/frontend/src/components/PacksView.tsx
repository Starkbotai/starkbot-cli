import { useState, useEffect, useMemo } from "react";
import type { PackInfo } from "../types";

interface Props {
  packs: PackInfo[];
  loading: boolean;
  message: string | null;
  onListPacks: () => void;
  onInstall: (slug: string) => void;
  onUninstall: (slug: string) => void;
}

export default function PacksView({ packs, loading, message, onListPacks, onInstall, onUninstall }: Props) {
  const [search, setSearch] = useState("");
  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);

  useEffect(() => {
    if (packs.length === 0 && !loading) {
      onListPacks();
    }
  }, []);

  const filtered = useMemo(() => {
    if (!search) return packs;
    const q = search.toLowerCase();
    return packs.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.slug.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q)
    );
  }, [packs, search]);

  const selected = useMemo(
    () => filtered.find((p) => p.slug === selectedSlug) ?? filtered[0] ?? null,
    [filtered, selectedSlug]
  );

  return (
    <div className="h-full flex">
      {/* Left: pack list */}
      <div className="w-2/5 border-r border-surface-3 flex flex-col">
        {/* Search bar */}
        <div className="p-2 border-b border-surface-3">
          <input
            type="text"
            value={search}
            onChange={(e) => { setSearch(e.target.value); setSelectedSlug(null); }}
            placeholder="Search packs..."
            className="w-full px-3 py-1.5 rounded bg-surface-2 border border-surface-3 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-accent"
          />
        </div>

        {/* Pack list */}
        <div className="flex-1 overflow-y-auto">
          {loading && filtered.length === 0 ? (
            <div className="p-4 text-sm text-yellow-400">Loading...</div>
          ) : filtered.length === 0 ? (
            <div className="p-4 text-sm text-gray-500">
              {packs.length === 0 ? (
                <button onClick={onListPacks} className="text-accent hover:underline">
                  Click to fetch packs from server
                </button>
              ) : (
                "No packs match your search"
              )}
            </div>
          ) : (
            filtered.map((pack) => (
              <button
                key={pack.slug}
                onClick={() => setSelectedSlug(pack.slug)}
                className={`w-full text-left px-3 py-2.5 border-b border-surface-2 transition-colors ${
                  selected?.slug === pack.slug
                    ? "bg-surface-2 border-l-2 border-l-accent"
                    : "hover:bg-surface-1"
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-gray-200">{pack.name}</span>
                  {pack.installed && (
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-green-900/40 text-green-400">
                      installed
                    </span>
                  )}
                </div>
                <div className="text-xs text-gray-500 mt-0.5 line-clamp-1">{pack.description}</div>
              </button>
            ))
          )}
        </div>

        {/* Bottom bar */}
        <div className="px-3 py-2 border-t border-surface-3 flex items-center gap-2">
          <button
            onClick={onListPacks}
            disabled={loading}
            className="px-3 py-1 rounded text-xs bg-surface-2 text-gray-300 hover:bg-surface-3 hover:text-white transition-colors disabled:opacity-50"
          >
            {loading ? "Loading..." : "Refresh"}
          </button>
          <span className="text-xs text-gray-500">
            {filtered.length} pack{filtered.length !== 1 ? "s" : ""}
          </span>
        </div>
      </div>

      {/* Right: detail pane */}
      <div className="w-3/5 p-6 overflow-y-auto">
        {selected ? (
          <>
            <div className="flex items-start justify-between mb-4">
              <div>
                <h2 className="text-xl font-semibold text-gray-100">{selected.name}</h2>
                <span className="text-xs text-gray-500">slug: {selected.slug}</span>
              </div>
              {selected.installed ? (
                <button
                  onClick={() => onUninstall(selected.slug)}
                  className="px-4 py-1.5 rounded text-sm bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors"
                >
                  Uninstall
                </button>
              ) : (
                <button
                  onClick={() => onInstall(selected.slug)}
                  disabled={loading}
                  className="px-4 py-1.5 rounded text-sm bg-accent/20 text-accent hover:bg-accent/30 transition-colors disabled:opacity-50"
                >
                  {loading ? "Installing..." : "Install"}
                </button>
              )}
            </div>

            <p className="text-sm text-gray-300 mb-4">{selected.description}</p>

            {selected.icon && (
              <div className="text-xs text-gray-500 mb-2">
                Icon: <span className="text-yellow-400">{selected.icon}</span>
              </div>
            )}

            <div className="mt-2">
              <span
                className={`inline-block text-xs px-2 py-1 rounded ${
                  selected.installed
                    ? "bg-green-900/30 text-green-400"
                    : "bg-surface-2 text-gray-500"
                }`}
              >
                {selected.installed ? "Installed" : "Not installed"}
              </span>
            </div>
          </>
        ) : (
          <div className="text-gray-500 text-sm">Select a pack to view details</div>
        )}

        {/* Status message */}
        {message && (
          <div
            className={`mt-6 text-sm px-3 py-2 rounded ${
              message.startsWith("Error")
                ? "bg-red-900/20 text-red-400"
                : "bg-surface-2 text-yellow-400"
            }`}
          >
            {message}
          </div>
        )}
      </div>
    </div>
  );
}
