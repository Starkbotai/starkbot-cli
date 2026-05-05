---
description: "Guide for checking and installing API keys for external services"
version: 1.0.0
tags: [configuration, security, api, infrastructure, devops]
requires_tools: [api_keys_check, install_api_key]
---

# API Key Management

## Checking Keys

Before making API calls to external services, always check if the required key is configured:

1. Use `api_keys_check` with `service_name` to check a specific key
2. Use `api_keys_check` without parameters to list all configured keys

## Installing Keys

When a user provides an API key:

1. Validate the service name is UPPER_SNAKE_CASE (e.g., `CLOUDFLARE_API_TOKEN`)
2. Use `install_api_key` with `service_name` and `api_key` parameters
3. Confirm successful installation (tool returns masked key)

## Common Service Names

- `CLOUDFLARE_API_TOKEN` — Cloudflare API bearer token
- `GITHUB_TOKEN` — GitHub personal access token
- `OPENAI_API_KEY` — OpenAI API key (usually set via env var)

## Security Notes

- Never display full API key values in output
- Keys are stored in the local SQLite database
- The `api_keys_check` tool never returns actual key values
- Always ask the user before storing a key
