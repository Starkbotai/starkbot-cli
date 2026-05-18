---
description: "Browse, install, and configure integrations and their API keys from chat"
version: 1.1.0
tags: [configuration, integrations, setup, onboarding]
requires_tools: [api_keys_check, install_integration, load_skill]
---

# Integration Setup

Help users discover available integrations, configure API keys, and install integrations — all from the chat.

## Available Integrations

| Integration | Preset ID | Required Keys | Where to Get Keys |
|---|---|---|---|
| **GitHub** | `github` | `GITHUB_TOKEN` | https://github.com/settings/tokens |
| **Cloudflare** | `cloudflare` | `CLOUDFLARE_API_TOKEN` | https://dash.cloudflare.com/profile/api-tokens |
| **Railway** | `railway` | `RAILWAY_TOKEN` | https://railway.com/account/tokens |
| **DigitalOcean** | `digitalocean` | `DIGITALOCEAN_TOKEN` | https://cloud.digitalocean.com/account/api/tokens |
| **AWS** | `aws` | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` (optional) | https://console.aws.amazon.com/iam/ |
| **Linear** | `linear` | `LINEAR_API_KEY` | https://linear.app/settings/api |
| **Notion** | `notion` | `NOTION_API_KEY` | https://www.notion.so/my-integrations |

## Workflow: Set Up an Integration

### Step 1: Check Current Status

List all configured keys to see what's already set up:
```
api_keys_check()
```

### Step 2: Collect the API Key

Ask the user for their API key. Provide the relevant link from the table above so they know where to create one.

Example prompt:
> To set up Linear, I need your API key. You can create one at https://linear.app/settings/api. Paste your key and I'll install the integration.

For multi-key integrations (AWS), collect all keys before installing.

### Step 3: Install the Integration

Use `install_integration` with the preset ID and all required API keys. This single call stores the keys, copies skill files, and registers the integration:

```
install_integration(preset_id: "linear", api_keys: [{"name": "LINEAR_API_KEY", "value": "<user-provided-key>"}])
```

For multi-key integrations:
```
install_integration(preset_id: "aws", api_keys: [{"name": "AWS_ACCESS_KEY_ID", "value": "<key>"}, {"name": "AWS_SECRET_ACCESS_KEY", "value": "<secret>"}])
```

The tool returns masked keys for confirmation and a list of installed skills. Never echo full keys back.

### Step 4: Use the Integration

After install, the integration's skills are available. Load one to start using it:
```
load_skill(skill_name: "linear-manage")
```

## Workflow: List What's Configured

When a user asks "what integrations do I have?" or "what's set up?":

1. Run `api_keys_check()` to list all stored keys
2. Cross-reference against the table above
3. Report which integrations are ready to use and which still need keys

## Workflow: Remove an Integration

If the user wants to remove an integration, let them know this can be done from the Integrations settings screen in the interface — there is no uninstall tool available in chat.

## Tips

- Always confirm with the user before storing a key
- For AWS, the region is optional — default is `us-east-1`
- Notion integrations must also be connected to specific pages in Notion's share menu
- After setup, offer to test the integration with a simple read-only operation (e.g., list issues, list zones, list droplets)
