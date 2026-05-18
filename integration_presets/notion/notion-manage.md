---
description: "Manage Notion pages, databases, and blocks via the REST API"
version: 1.0.0
tags: [productivity, notion, knowledge-base, docs]
requires_tools: [web_fetch, api_keys_check]
---

# Notion Management

## Prerequisites

Requires API key: `NOTION_API_KEY`

Before any operation, check the key exists:
```
api_keys_check(service_name: "NOTION_API_KEY")
```

If not configured, ask the user to provide it and install via `install_integration`.
Integrations are created at https://www.notion.so/my-integrations

**Note**: The Notion integration must be connected to specific pages/databases in Notion's share menu before it can access them.

## API Base

All requests go to `https://api.notion.com/v1/`

Use `web_fetch` with:
- `auth: "bearer:NOTION_API_KEY"` — this resolves the token from the keystore automatically
- `headers: {"Content-Type": "application/json", "Notion-Version": "2022-06-28"}`
- Appropriate `method` (GET, POST, PATCH, DELETE)

**IMPORTANT**: Never manually construct Authorization headers. Always use the `auth` parameter. Always include the `Notion-Version` header.

---

## Search

### Search All Pages & Databases
```
POST /search
Body: {
  "query": "search terms",
  "filter": {"property": "object", "value": "page"},
  "sort": {"direction": "descending", "timestamp": "last_edited_time"},
  "page_size": 10
}
```
Filter `value` can be `"page"` or `"database"`.

---

## Pages

### Get Page
```
GET /pages/{page_id}
```

### Create Page
```
POST /pages
Body: {
  "parent": {"database_id": "..."},
  "properties": {
    "Name": {"title": [{"text": {"content": "Page Title"}}]},
    "Status": {"select": {"name": "In Progress"}}
  },
  "children": [
    {
      "object": "block",
      "type": "paragraph",
      "paragraph": {
        "rich_text": [{"type": "text", "text": {"content": "Page body text"}}]
      }
    }
  ]
}
```

To create a page under another page (not in a database):
```json
{"parent": {"page_id": "..."}, "properties": {"title": {"title": [{"text": {"content": "Sub-page"}}]}}}
```

### Update Page Properties
```
PATCH /pages/{page_id}
Body: {
  "properties": {
    "Status": {"select": {"name": "Done"}}
  }
}
```

### Archive Page
```
PATCH /pages/{page_id}
Body: {"archived": true}
```

---

## Databases

### Get Database
```
GET /databases/{database_id}
```

### Query Database
```
POST /databases/{database_id}/query
Body: {
  "filter": {
    "property": "Status",
    "select": {"equals": "In Progress"}
  },
  "sorts": [{"property": "Created", "direction": "descending"}],
  "page_size": 20
}
```

Compound filters:
```json
{
  "filter": {
    "and": [
      {"property": "Status", "select": {"equals": "In Progress"}},
      {"property": "Assignee", "people": {"contains": "user-id"}}
    ]
  }
}
```

### Create Database
```
POST /databases
Body: {
  "parent": {"page_id": "..."},
  "title": [{"type": "text", "text": {"content": "Task Tracker"}}],
  "properties": {
    "Name": {"title": {}},
    "Status": {"select": {"options": [{"name": "To Do"}, {"name": "In Progress"}, {"name": "Done"}]}},
    "Priority": {"select": {"options": [{"name": "High"}, {"name": "Medium"}, {"name": "Low"}]}}
  }
}
```

### Create Database Entry
Use "Create Page" with `"parent": {"database_id": "..."}` and set properties matching the database schema.

---

## Blocks

### Get Block Children
```
GET /blocks/{block_id}/children?page_size=100
```
Use the page ID to get a page's content blocks.

### Append Block Children
```
PATCH /blocks/{block_id}/children
Body: {
  "children": [
    {
      "object": "block",
      "type": "heading_2",
      "heading_2": {
        "rich_text": [{"type": "text", "text": {"content": "Section Title"}}]
      }
    },
    {
      "object": "block",
      "type": "paragraph",
      "paragraph": {
        "rich_text": [{"type": "text", "text": {"content": "Paragraph text"}}]
      }
    },
    {
      "object": "block",
      "type": "to_do",
      "to_do": {
        "rich_text": [{"type": "text", "text": {"content": "Task item"}}],
        "checked": false
      }
    }
  ]
}
```

### Delete Block
```
DELETE /blocks/{block_id}
```

---

## Common Block Types

- `paragraph` — text content
- `heading_1`, `heading_2`, `heading_3` — headings
- `bulleted_list_item`, `numbered_list_item` — list items
- `to_do` — checkbox items
- `code` — code blocks (include `language` field)
- `quote` — blockquotes
- `divider` — horizontal rule
- `callout` — callout boxes (include `icon`)

---

## Workflow

1. Check API key exists
2. Search for pages/databases or use known IDs
3. Perform requested operations
4. Always confirm destructive actions (archive, delete) with the user
5. Report results clearly with page/database titles and URLs
