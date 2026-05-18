---
description: "Deploy and manage applications on Railway via the GraphQL API"
version: 1.0.0
tags: [infrastructure, deploy, railway, devops]
requires_tools: [web_fetch, api_keys_check]
---

# Railway Management

## Prerequisites

Requires API key: `RAILWAY_TOKEN`

Before any operation, check the key exists:
```
api_keys_check(service_name: "RAILWAY_TOKEN")
```

If not configured, ask the user to provide it and install via `install_integration`.
Tokens are created at https://railway.com/account/tokens

## API Endpoint

All requests go to `https://backboard.railway.com/graphql/v2`

Use `web_fetch` with:
- `method: "POST"`
- `auth: "bearer:RAILWAY_TOKEN"` — this resolves the token from the keystore automatically
- `headers: {"Content-Type": "application/json"}`
- `body` containing `{"query": "...", "variables": {...}}`

**IMPORTANT**: Never manually construct Authorization headers. Always use the `auth` parameter.

## Rate Limits

- Free: 100 req/hr
- Hobby: 1,000 req/hr
- Pro: 10,000 req/hr

---

## Projects

### List All Projects
```graphql
query {
  projects {
    edges {
      node { id name description createdAt updatedAt }
    }
  }
}
```

### Get Project (with services & environments)
```graphql
query project($id: String!) {
  project(id: $id) {
    id name description createdAt
    services { edges { node { id name icon } } }
    environments { edges { node { id name } } }
  }
}
```

### Create Project
```graphql
mutation projectCreate($input: ProjectCreateInput!) {
  projectCreate(input: $input) { id name }
}
```
Variables: `{ "input": { "name": "My Project" } }`

### Update Project
```graphql
mutation projectUpdate($id: String!, $input: ProjectUpdateInput!) {
  projectUpdate(id: $id, input: $input) { id name description }
}
```

### Delete Project
```graphql
mutation projectDelete($id: String!) {
  projectDelete(id: $id)
}
```

---

## Services

### Get Service Details
```graphql
query service($id: String!) {
  service(id: $id) { id name icon createdAt projectId }
}
```

### Get Service Instance (config & latest deployment)
```graphql
query serviceInstance($serviceId: String!, $environmentId: String!) {
  serviceInstance(serviceId: $serviceId, environmentId: $environmentId) {
    id serviceName startCommand buildCommand rootDirectory
    healthcheckPath region numReplicas
    restartPolicyType restartPolicyMaxRetries
    latestDeployment { id status createdAt }
  }
}
```

### Create Service (from GitHub repo)
```graphql
mutation serviceCreate($input: ServiceCreateInput!) {
  serviceCreate(input: $input) { id name }
}
```
Variables: `{ "input": { "projectId": "...", "name": "my-service", "source": { "repo": "owner/repo", "branch": "main" } } }`

### Create Service (from Docker image)
Same mutation with: `"source": { "image": "redis:7-alpine" }`

### Update Service Instance Settings
```graphql
mutation serviceInstanceUpdate($serviceId: String!, $environmentId: String!, $input: ServiceInstanceUpdateInput!) {
  serviceInstanceUpdate(serviceId: $serviceId, environmentId: $environmentId, input: $input)
}
```
Options: `startCommand, buildCommand, rootDirectory, healthcheckPath, region, numReplicas, restartPolicyType, cronSchedule, dockerfilePath, watchPatterns`

### Deploy Service
```graphql
mutation serviceInstanceDeployV2($serviceId: String!, $environmentId: String!) {
  serviceInstanceDeployV2(serviceId: $serviceId, environmentId: $environmentId)
}
```

### Redeploy Latest
```graphql
mutation serviceInstanceRedeploy($serviceId: String!, $environmentId: String!) {
  serviceInstanceRedeploy(serviceId: $serviceId, environmentId: $environmentId)
}
```

### Delete Service
```graphql
mutation serviceDelete($id: String!) {
  serviceDelete(id: $id)
}
```

---

## Deployments

### List Deployments
```graphql
query deployments($input: DeploymentListInput!, $first: Int) {
  deployments(input: $input, first: $first) {
    edges {
      node { id status createdAt url staticUrl }
    }
  }
}
```
Variables: `{ "input": { "projectId": "...", "serviceId": "...", "environmentId": "..." }, "first": 10 }`

### Get Latest Successful Deployment
Same query with: `"input": { ..., "status": { "successfulOnly": true } }` and `"first": 1`

### Get Deployment Details
```graphql
query deployment($id: String!) {
  deployment(id: $id) {
    id status createdAt url staticUrl meta canRedeploy canRollback
  }
}
```

### Build Logs
```graphql
query buildLogs($deploymentId: String!, $limit: Int) {
  buildLogs(deploymentId: $deploymentId, limit: $limit) {
    timestamp message severity
  }
}
```

### Runtime Logs
```graphql
query deploymentLogs($deploymentId: String!, $limit: Int) {
  deploymentLogs(deploymentId: $deploymentId, limit: $limit) {
    timestamp message severity
  }
}
```

### HTTP Request Logs
```graphql
query httpLogs($deploymentId: String!, $limit: Int) {
  httpLogs(deploymentId: $deploymentId, limit: $limit) {
    timestamp method path httpStatus totalDuration srcIp
  }
}
```

### Redeploy / Restart / Rollback / Stop / Cancel
```graphql
mutation deploymentRedeploy($id: String!) { deploymentRedeploy(id: $id) { id status } }
mutation deploymentRestart($id: String!) { deploymentRestart(id: $id) }
mutation deploymentRollback($id: String!) { deploymentRollback(id: $id) { id status } }
mutation deploymentStop($id: String!) { deploymentStop(id: $id) }
mutation deploymentCancel($id: String!) { deploymentCancel(id: $id) }
```

---

## Variables

### Get Variables
```graphql
query variables($projectId: String!, $environmentId: String!, $serviceId: String) {
  variables(projectId: $projectId, environmentId: $environmentId, serviceId: $serviceId)
}
```

### Set Variable
```graphql
mutation variableUpsert($input: VariableUpsertInput!) {
  variableUpsert(input: $input)
}
```
Variables: `{ "input": { "projectId": "...", "environmentId": "...", "serviceId": "...", "name": "KEY", "value": "value" } }`

### Set Multiple Variables
```graphql
mutation variableCollectionUpsert($input: VariableCollectionUpsertInput!) {
  variableCollectionUpsert(input: $input)
}
```
Variables: `{ "input": { "projectId": "...", "environmentId": "...", "serviceId": "...", "variables": { "KEY1": "val1", "KEY2": "val2" } } }`

### Delete Variable
```graphql
mutation variableDelete($input: VariableDeleteInput!) {
  variableDelete(input: $input)
}
```
Variables: `{ "input": { "projectId": "...", "environmentId": "...", "serviceId": "...", "name": "OLD_VAR" } }`

---

## Workflow

1. Check API key exists
2. List projects to find the target project ID
3. Get project details to find service IDs and environment IDs
4. Perform operations using those IDs
5. Always confirm destructive actions (delete, stop, rollback) with the user
6. Report results clearly with deployment URLs and status
