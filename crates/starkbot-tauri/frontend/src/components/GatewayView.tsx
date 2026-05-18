import { useState, useEffect } from "react";
import type { ChannelInfo, ChannelSettingInfo } from "../types";

const CHANNEL_TYPES = [
  { value: "custom", label: "Custom HTTP" },
  { value: "discord", label: "Discord" },
  { value: "telegram", label: "Telegram" },
];

interface Props {
  channels: ChannelInfo[];
  channelSettings: ChannelSettingInfo[];
  onCreateChannel: (channelType: string, name: string) => void;
  onDeleteChannel: (channelId: string) => void;
  onStartChannel: (channelId: string) => void;
  onStopChannel: (channelId: string) => void;
  onUpdateSetting: (channelId: string, key: string, value: string) => void;
  onLoadSettings: (channelId: string) => void;
  onListChannels: () => void;
}

export default function GatewayView({
  channels,
  channelSettings,
  onCreateChannel,
  onDeleteChannel,
  onStartChannel,
  onStopChannel,
  onUpdateSetting,
  onLoadSettings,
  onListChannels,
}: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [createType, setCreateType] = useState("custom");
  const [createName, setCreateName] = useState("");
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  useEffect(() => {
    onListChannels();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (selectedId && channels.find((c) => c.id === selectedId)) {
      onLoadSettings(selectedId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedId]);

  const selected = channels.find((c) => c.id === selectedId);

  const handleCreate = () => {
    const name = createName.trim() || `My ${CHANNEL_TYPES.find((t) => t.value === createType)?.label} Channel`;
    onCreateChannel(createType, name);
    setShowCreate(false);
    setCreateName("");
    setCreateType("custom");
  };

  const handleSaveSetting = (key: string) => {
    if (selectedId) {
      onUpdateSetting(selectedId, key, editValue);
      setEditingKey(null);
      setEditValue("");
    }
  };

  const generateToken = () => {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  };

  return (
    <div className="flex h-full">
      {/* Left: Channel list */}
      <div className="w-64 border-r border-surface-3 flex flex-col">
        <div className="flex items-center justify-between px-3 py-2 border-b border-surface-3">
          <span className="text-sm font-medium text-gray-300">Channels</span>
          <button
            onClick={() => setShowCreate(true)}
            className="px-2 py-0.5 text-xs rounded bg-accent text-white hover:bg-accent/80"
          >
            + New
          </button>
        </div>

        {showCreate && (
          <div className="p-3 border-b border-surface-3 bg-surface-1 space-y-2">
            <select
              value={createType}
              onChange={(e) => setCreateType(e.target.value)}
              className="w-full px-2 py-1 text-sm bg-surface-2 border border-surface-3 rounded text-gray-200"
            >
              {CHANNEL_TYPES.map((t) => (
                <option key={t.value} value={t.value} className="bg-surface-2 text-gray-200">{t.label}</option>
              ))}
            </select>
            <input
              type="text"
              placeholder="Channel name"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleCreate()}
              className="w-full px-2 py-1 text-sm bg-surface-2 border border-surface-3 rounded text-gray-200 placeholder-gray-500"
              autoFocus
            />
            <div className="flex gap-2">
              <button
                onClick={handleCreate}
                className="flex-1 px-2 py-1 text-xs rounded bg-accent text-white hover:bg-accent/80"
              >
                Create
              </button>
              <button
                onClick={() => setShowCreate(false)}
                className="flex-1 px-2 py-1 text-xs rounded bg-surface-2 text-gray-400 hover:text-gray-200"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto">
          {channels.length === 0 ? (
            <div className="p-4 text-center text-sm text-gray-500">
              No channels yet. Create one to get started.
            </div>
          ) : (
            channels.map((ch) => (
              <button
                key={ch.id}
                onClick={() => setSelectedId(ch.id)}
                className={`w-full px-3 py-2 text-left text-sm flex items-center gap-2 transition-colors ${
                  selectedId === ch.id
                    ? "bg-surface-2 text-white"
                    : "text-gray-400 hover:bg-surface-1 hover:text-gray-200"
                }`}
              >
                <span className={`w-2 h-2 rounded-full ${ch.running ? "bg-green-400" : "bg-gray-600"}`} />
                <span className="flex-1 truncate">{ch.name}</span>
                <span className="text-[10px] text-gray-500 uppercase">{ch.channel_type}</span>
              </button>
            ))
          )}
        </div>
      </div>

      {/* Middle + Right: Settings & Actions */}
      {selected ? (
        <div className="flex-1 flex">
          {/* Settings */}
          <div className="flex-1 border-r border-surface-3 p-4 overflow-y-auto">
            <h3 className="text-sm font-medium text-gray-300 mb-3">Settings</h3>
            <div className="space-y-3">
              {channelSettings.map((setting) => (
                <div key={setting.key}>
                  <label className="block text-xs text-gray-500 mb-1">{setting.label}</label>
                  {setting.key === "auth_token" ? (
                    <div className="flex items-center gap-2">
                      <span className="flex-1 px-2 py-1 text-sm bg-surface-2 border border-surface-3 rounded text-gray-400 font-mono truncate">
                        {setting.value ? "****" : "(not set)"}
                      </span>
                      <button
                        onClick={() => {
                          const token = generateToken();
                          onUpdateSetting(selected.id, setting.key, token);
                        }}
                        className="px-3 py-1 text-xs rounded bg-accent text-white hover:bg-accent/80 whitespace-nowrap"
                      >
                        {setting.value ? "Regenerate" : "Generate"}
                      </button>
                    </div>
                  ) : setting.input_type === "toggle" ? (
                    <button
                      onClick={() => {
                        const newVal = setting.value === "1" ? "0" : "1";
                        onUpdateSetting(selected.id, setting.key, newVal);
                      }}
                      className={`px-3 py-1 text-xs rounded ${
                        setting.value === "1"
                          ? "bg-green-600 text-white"
                          : "bg-surface-2 text-gray-400"
                      }`}
                    >
                      {setting.value === "1" ? "ON" : "OFF"}
                    </button>
                  ) : editingKey === setting.key ? (
                    <div className="flex gap-2">
                      <input
                        type={setting.input_type === "password" ? "password" : "text"}
                        value={editValue}
                        onChange={(e) => setEditValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") handleSaveSetting(setting.key);
                          if (e.key === "Escape") setEditingKey(null);
                        }}
                        className="flex-1 px-2 py-1 text-sm bg-surface-2 border border-surface-3 rounded text-gray-200"
                        autoFocus
                      />
                      <button
                        onClick={() => handleSaveSetting(setting.key)}
                        className="px-2 py-1 text-xs rounded bg-accent text-white"
                      >
                        Save
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => {
                        setEditingKey(setting.key);
                        setEditValue(setting.value);
                      }}
                      className="w-full text-left px-2 py-1 text-sm bg-surface-2 border border-surface-3 rounded text-gray-300 hover:border-accent/50"
                    >
                      {setting.input_type === "password" && setting.value
                        ? "****"
                        : setting.value || "(not set)"}
                    </button>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Actions */}
          <div className="w-56 p-4">
            <h3 className="text-sm font-medium text-gray-300 mb-3">Info</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">Type</span>
                <span className="text-gray-300 capitalize">{selected.channel_type}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Status</span>
                <span className={selected.running ? "text-green-400" : "text-gray-500"}>
                  {selected.running ? "Running" : "Stopped"}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">Safe Mode</span>
                <span className="text-gray-300">{selected.safe_mode ? "ON" : "OFF"}</span>
              </div>
            </div>

            <div className="mt-6 space-y-2">
              {selected.running ? (
                <button
                  onClick={() => onStopChannel(selected.id)}
                  className="w-full px-3 py-1.5 text-sm rounded bg-red-600/20 text-red-400 hover:bg-red-600/30 border border-red-600/30"
                >
                  Stop Channel
                </button>
              ) : (
                <button
                  onClick={() => onStartChannel(selected.id)}
                  className="w-full px-3 py-1.5 text-sm rounded bg-green-600/20 text-green-400 hover:bg-green-600/30 border border-green-600/30"
                >
                  Start Channel
                </button>
              )}
              <button
                onClick={() => {
                  onDeleteChannel(selected.id);
                  setSelectedId(null);
                }}
                className="w-full px-3 py-1.5 text-sm rounded bg-surface-2 text-gray-400 hover:text-red-400 hover:bg-red-600/10 border border-surface-3"
              >
                Delete Channel
              </button>
            </div>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center text-gray-500 text-sm">
          Select a channel or create a new one
        </div>
      )}
    </div>
  );
}
