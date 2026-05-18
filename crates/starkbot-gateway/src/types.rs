use serde::{Deserialize, Serialize};

/// The type of gateway channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    Custom,
    Discord,
    Telegram,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Discord => "discord",
            Self::Telegram => "telegram",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Custom => "Custom HTTP",
            Self::Discord => "Discord",
            Self::Telegram => "Telegram",
        }
    }

    pub fn all() -> &'static [ChannelType] {
        &[ChannelType::Custom, ChannelType::Discord, ChannelType::Telegram]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "custom" => Some(Self::Custom),
            "discord" => Some(Self::Discord),
            "telegram" => Some(Self::Telegram),
            _ => None,
        }
    }

    pub fn setting_keys(&self) -> &'static [ChannelSettingKey] {
        match self {
            Self::Custom => &[ChannelSettingKey::ListenPort, ChannelSettingKey::AuthToken, ChannelSettingKey::SafeMode],
            Self::Discord => &[ChannelSettingKey::DiscordBotToken, ChannelSettingKey::SafeMode],
            Self::Telegram => &[ChannelSettingKey::TelegramBotToken, ChannelSettingKey::SafeMode],
        }
    }
}

/// A gateway channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub channel_type: ChannelType,
    pub name: String,
    pub enabled: bool,
    pub safe_mode: bool,
    pub created_at: String,
}

/// Setting keys for channel configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelSettingKey {
    ListenPort,
    AuthToken,
    DiscordBotToken,
    TelegramBotToken,
    SafeMode,
}

impl ChannelSettingKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ListenPort => "listen_port",
            Self::AuthToken => "auth_token",
            Self::DiscordBotToken => "discord_bot_token",
            Self::TelegramBotToken => "telegram_bot_token",
            Self::SafeMode => "safe_mode",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ListenPort => "Listen Port",
            Self::AuthToken => "Auth Token",
            Self::DiscordBotToken => "Discord Bot Token",
            Self::TelegramBotToken => "Telegram Bot Token",
            Self::SafeMode => "Safe Mode",
        }
    }

    pub fn input_type(&self) -> &'static str {
        match self {
            Self::ListenPort => "number",
            Self::AuthToken | Self::DiscordBotToken | Self::TelegramBotToken => "password",
            Self::SafeMode => "toggle",
        }
    }
}

/// A message normalized from any channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub channel_id: String,
    pub channel_type: ChannelType,
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub message_id: String,
    pub safe_mode: bool,
}

/// Handle for a running channel, used to shut it down.
pub struct ChannelHandle {
    pub channel_id: String,
    pub channel_type: ChannelType,
    pub name: String,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}
