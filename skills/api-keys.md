---
description: "Guide for checking and installing API keys for external services"
version: 1.0.0
tags: [configuration, security, api, infrastructure, devops]
requires_tools: [api_keys_check, install_integration]
---

# API Key Management

## Checking Keys

Before making API calls to external services, always check if the required key is configured:

1. Use `api_keys_check` with `service_name` to check a specific key
2. Use `api_keys_check` without parameters to list all configured keys

## Installing Keys

When a user provides an API key for an integration:

1. Use `install_integration` with `preset_id` and `api_keys` parameters (e.g., `install_integration(preset_id: "linear", api_keys: [{"name": "LINEAR_API_KEY", "value": "..."}])`). This stores the key, copies skill files, and registers the integration in one step.
2. Confirm successful installation (tool returns masked keys and installed skills)

## Common Service Names

- `GITHUB_TOKEN` — GitHub personal access token
- `CLOUDFLARE_API_TOKEN` — Cloudflare API bearer token
- `RAILWAY_TOKEN` — Railway deployment platform token
- `DIGITALOCEAN_TOKEN` — DigitalOcean API bearer token
- `AWS_ACCESS_KEY_ID` — AWS access key ID
- `AWS_SECRET_ACCESS_KEY` — AWS secret access key
- `AWS_REGION` — AWS region (optional, defaults to us-east-1)
- `LINEAR_API_KEY` — Linear project management API key
- `NOTION_API_KEY` — Notion integration API key
- `OPENAI_API_KEY` — OpenAI API key (usually set via env var)

## Security Notes

- Never display full API key values in output
- Keys are stored in the local SQLite database
- The `api_keys_check` tool never returns actual key values
- Always ask the user before storing a key
