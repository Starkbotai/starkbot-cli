---
description: "Manage Linear issues, projects, teams, and cycles via the GraphQL API"
version: 1.0.0
tags: [project-management, linear, issues, productivity]
requires_tools: [web_fetch, api_keys_check]
---

# Linear Management

## Prerequisites

Requires API key: `LINEAR_API_KEY`

Before any operation, check the key exists:
```
api_keys_check(service_name: "LINEAR_API_KEY")
```

If not configured, ask the user to provide it and install via `install_integration`.
Keys are created at https://linear.app/settings/api

## API Endpoint

All requests go to `https://api.linear.app/graphql`

Use `web_fetch` with:
- `method: "POST"`
- `auth: "bearer:LINEAR_API_KEY"` — this resolves the token from the keystore automatically
- `headers: {"Content-Type": "application/json"}`
- `body` containing `{"query": "...", "variables": {...}}`

**IMPORTANT**: Never manually construct Authorization headers. Always use the `auth` parameter.

---

## Teams

### List Teams
```graphql
query {
  teams {
    nodes { id name key description }
  }
}
```

---

## Issues

### List Issues
```graphql
query issues($filter: IssueFilter, $first: Int) {
  issues(filter: $filter, first: $first) {
    nodes {
      id identifier title state { name } priority assignee { name } createdAt
    }
  }
}
```
Filter examples:
- By team: `{"team": {"key": {"eq": "ENG"}}}`
- By state: `{"state": {"name": {"eq": "In Progress"}}}`
- By assignee: `{"assignee": {"email": {"eq": "user@example.com"}}}`

### Get Issue
```graphql
query issue($id: String!) {
  issue(id: $id) {
    id identifier title description state { name }
    priority assignee { name } project { name }
    labels { nodes { name } }
    comments { nodes { body user { name } createdAt } }
  }
}
```

### Create Issue
```graphql
mutation issueCreate($input: IssueCreateInput!) {
  issueCreate(input: $input) {
    success
    issue { id identifier title url }
  }
}
```
Variables: `{"input": {"teamId": "...", "title": "Bug: ...", "description": "...", "priority": 2}}`

Priority values: 0 = No priority, 1 = Urgent, 2 = High, 3 = Medium, 4 = Low

### Update Issue
```graphql
mutation issueUpdate($id: String!, $input: IssueUpdateInput!) {
  issueUpdate(id: $id, input: $input) {
    success
    issue { id identifier title state { name } }
  }
}
```
Variables: `{"id": "...", "input": {"stateId": "...", "assigneeId": "...", "priority": 1}}`

### Close Issue
To close an issue, update its state to a "Done" or "Canceled" state. First query the team's workflow states:
```graphql
query workflowStates($teamId: String!) {
  workflowStates(filter: {team: {id: {eq: $teamId}}}) {
    nodes { id name type }
  }
}
```
Then update the issue with the completed state ID.

### Add Comment
```graphql
mutation commentCreate($input: CommentCreateInput!) {
  commentCreate(input: $input) {
    success
    comment { id body }
  }
}
```
Variables: `{"input": {"issueId": "...", "body": "Comment text here"}}`

### Assign Issue
Use `issueUpdate` with `{"input": {"assigneeId": "user-id"}}`.

To find a user ID:
```graphql
query {
  users { nodes { id name email } }
}
```

---

## Projects

### List Projects
```graphql
query {
  projects {
    nodes { id name description state startDate targetDate }
  }
}
```

### Create Project
```graphql
mutation projectCreate($input: ProjectCreateInput!) {
  projectCreate(input: $input) {
    success
    project { id name url }
  }
}
```
Variables: `{"input": {"name": "Project Name", "teamIds": ["..."], "description": "..."}}`

---

## Cycles

### List Active Cycle
```graphql
query activeCycle($teamId: String!) {
  team(id: $teamId) {
    activeCycle {
      id name startsAt endsAt
      issues { nodes { id identifier title state { name } } }
    }
  }
}
```

### List Cycles
```graphql
query cycles($teamId: String!) {
  team(id: $teamId) {
    cycles {
      nodes { id name startsAt endsAt completedAt }
    }
  }
}
```

---

## Workflow

1. Check API key exists
2. List teams to get team IDs
3. Perform requested operations using team/issue/project IDs
4. Always confirm destructive actions with the user
5. Report results clearly with issue identifiers and URLs
