---
description: "Manage Cloudflare zones, DNS records, and redirect rules via the Cloudflare API"
version: 1.1.0
tags: [infrastructure, dns, cloudflare, devops]
requires_tools: [web_fetch, api_keys_check]
---

# Cloudflare DNS Management

## Prerequisites

Requires API key: `CLOUDFLARE_API_TOKEN`

Before any operation, check the key exists:
```
api_keys_check(service_name: "CLOUDFLARE_API_TOKEN")
```

If not configured, ask the user to provide it and install via `install_api_key`.

## API Base

All requests go to `https://api.cloudflare.com/client/v4/`

Use `web_fetch` with:
- `auth: "bearer:CLOUDFLARE_API_TOKEN"` — this resolves the token from the keystore automatically
- `headers: {"Content-Type": "application/json"}` for requests with a body
- Appropriate `method` (GET, POST, PUT, DELETE, PATCH)

**IMPORTANT**: Never manually construct Authorization headers. Always use the `auth` parameter.

## Common Operations

### List Zones
```
GET /zones
```
Returns all zones (domains) on the account.

### List DNS Records
```
GET /zones/{zone_id}/dns_records
```
Optional query params: `?type=A&name=example.com`

### Create DNS Record
```
POST /zones/{zone_id}/dns_records
Body: {"type": "A", "name": "sub.example.com", "content": "1.2.3.4", "ttl": 1, "proxied": true}
```

### Update DNS Record
```
PUT /zones/{zone_id}/dns_records/{record_id}
Body: {"type": "A", "name": "sub.example.com", "content": "5.6.7.8", "ttl": 1, "proxied": true}
```

### Delete DNS Record
```
DELETE /zones/{zone_id}/dns_records/{record_id}
```

### Redirect Rules (Page Rules)
```
GET /zones/{zone_id}/pagerules
POST /zones/{zone_id}/pagerules
Body: {"targets": [{"target": "url", "constraint": {"operator": "matches", "value": "*.example.com/*"}}], "actions": [{"id": "forwarding_url", "value": {"url": "https://new.example.com/$2", "status_code": 301}}], "status": "active"}
```

## Workflow

1. Check API key exists
2. List zones to get zone_id
3. Perform DNS operations using zone_id
4. Always confirm changes with the user before creating/updating/deleting records
5. Report results clearly with record details
