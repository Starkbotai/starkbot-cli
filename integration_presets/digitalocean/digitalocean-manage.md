---
description: "Manage DigitalOcean droplets, domains, apps, and databases via the DO API"
version: 1.0.0
tags: [infrastructure, cloud, digitalocean, devops]
requires_tools: [web_fetch, api_keys_check]
---

# DigitalOcean Management

## Prerequisites

Requires API key: `DIGITALOCEAN_TOKEN`

Before any operation, check the key exists:
```
api_keys_check(service_name: "DIGITALOCEAN_TOKEN")
```

If not configured, ask the user to provide it and install via `install_integration`.
Tokens are created at https://cloud.digitalocean.com/account/api/tokens

## API Base

All requests go to `https://api.digitalocean.com/v2/`

Use `web_fetch` with:
- `auth: "bearer:DIGITALOCEAN_TOKEN"` — this resolves the token from the keystore automatically
- `headers: {"Content-Type": "application/json"}` for requests with a body
- Appropriate `method` (GET, POST, PUT, DELETE)

**IMPORTANT**: Never manually construct Authorization headers. Always use the `auth` parameter.

## Pagination

Most list endpoints support `?page=1&per_page=20`. Response includes a `links.pages` object with `next`/`last` URLs and a `meta.total` count.

---

## Droplets

### List Droplets
```
GET /droplets
```
Optional: `?tag_name=web` to filter by tag.

### Get Droplet
```
GET /droplets/{droplet_id}
```

### Create Droplet
```
POST /droplets
Body: {
  "name": "my-droplet",
  "region": "nyc3",
  "size": "s-1vcpu-1gb",
  "image": "ubuntu-24-04-x64",
  "ssh_keys": ["fingerprint_or_id"],
  "tags": ["web"]
}
```

### Delete Droplet
```
DELETE /droplets/{droplet_id}
```

### Droplet Actions

Power on/off, reboot, resize:
```
POST /droplets/{droplet_id}/actions
Body: {"type": "power_off"}
Body: {"type": "power_on"}
Body: {"type": "reboot"}
Body: {"type": "resize", "size": "s-2vcpu-2gb", "disk": true}
```

---

## Domains & DNS Records

### List Domains
```
GET /domains
```

### Create Domain
```
POST /domains
Body: {"name": "example.com"}
```

### List DNS Records
```
GET /domains/{domain_name}/records
```

### Create DNS Record
```
POST /domains/{domain_name}/records
Body: {"type": "A", "name": "sub", "data": "1.2.3.4", "ttl": 1800}
```

### Update DNS Record
```
PUT /domains/{domain_name}/records/{record_id}
Body: {"data": "5.6.7.8"}
```

### Delete DNS Record
```
DELETE /domains/{domain_name}/records/{record_id}
```

---

## Apps (App Platform)

### List Apps
```
GET /apps
```

### Get App
```
GET /apps/{app_id}
```

### Create App
```
POST /apps
Body: {
  "spec": {
    "name": "my-app",
    "services": [{
      "name": "web",
      "github": {"repo": "owner/repo", "branch": "main"},
      "run_command": "npm start",
      "http_port": 8080,
      "instance_size_slug": "apps-s-1vcpu-0.5gb",
      "instance_count": 1
    }]
  }
}
```

### Deploy App
```
POST /apps/{app_id}/deployments
Body: {"force_build": true}
```

### Delete App
```
DELETE /apps/{app_id}
```

---

## Managed Databases

### List Databases
```
GET /databases
```

### Get Database
```
GET /databases/{database_cluster_id}
```
Returns connection details including `host`, `port`, `user`, `password`, `uri`.

### List Database Connection Pools
```
GET /databases/{database_cluster_id}/pools
```

---

## Tags

### List Tags
```
GET /tags
```

### Tag Resources
```
POST /tags/{tag_name}/resources
Body: {"resources": [{"resource_id": "droplet_id", "resource_type": "droplet"}]}
```

---

## Workflow

1. Check API key exists
2. Perform requested operations
3. Always confirm destructive actions (delete, power off, resize) with the user
4. Report results clearly with resource IDs, IPs, and status
