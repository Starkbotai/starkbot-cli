---
description: "Manage AWS Lambda functions via the AWS CLI"
version: 1.0.0
tags: [infrastructure, cloud, aws, devops]
requires_tools: [bash, api_keys_check]
---

# AWS Lambda Management

## Prerequisites

Requires API keys: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_REGION`.

Before any operation, check the keys exist:
```
api_keys_check(service_name: "AWS_ACCESS_KEY_ID")
api_keys_check(service_name: "AWS_SECRET_ACCESS_KEY")
```

If not configured, ask the user to provide them and install via `install_integration`.

## CLI Setup

AWS operations use the `aws` CLI tool via `bash`. Before running commands, export credentials:
```bash
export AWS_ACCESS_KEY_ID="<resolved from keystore>"
export AWS_SECRET_ACCESS_KEY="<resolved from keystore>"
export AWS_REGION="${AWS_REGION:-us-east-1}"
```

**IMPORTANT**: Use `api_key_read` to retrieve key values, then pass them as environment variables to `bash`. Never log or echo secret keys.

---

## Functions

### List Functions
```bash
aws lambda list-functions --output table
```

### Get Function Details
```bash
aws lambda get-function --function-name my-function
```

### Get Function Configuration
```bash
aws lambda get-function-configuration --function-name my-function
```

### Create Function (from ZIP)
```bash
aws lambda create-function \
  --function-name my-function \
  --runtime nodejs20.x \
  --role arn:aws:iam::ACCOUNT_ID:role/lambda-role \
  --handler index.handler \
  --zip-file fileb://function.zip
```

### Update Function Code
```bash
aws lambda update-function-code \
  --function-name my-function \
  --zip-file fileb://function.zip
```

### Update Function Configuration
```bash
aws lambda update-function-configuration \
  --function-name my-function \
  --timeout 30 \
  --memory-size 256 \
  --environment "Variables={KEY1=value1,KEY2=value2}"
```

### Delete Function
```bash
aws lambda delete-function --function-name my-function
```

---

## Invocation

### Invoke Function
```bash
aws lambda invoke \
  --function-name my-function \
  --payload '{"key": "value"}' \
  --cli-binary-format raw-in-base64-out \
  output.json
cat output.json
```

### Invoke with Log Output
```bash
aws lambda invoke \
  --function-name my-function \
  --payload '{"key": "value"}' \
  --cli-binary-format raw-in-base64-out \
  --log-type Tail \
  output.json \
  --query 'LogResult' --output text | base64 -d
```

---

## Logs

### View Recent Logs
```bash
aws logs describe-log-groups --log-group-name-prefix /aws/lambda/my-function
aws logs tail /aws/lambda/my-function --since 1h
```

### Follow Logs (streaming)
```bash
aws logs tail /aws/lambda/my-function --follow
```

---

## Aliases & Versions

### List Versions
```bash
aws lambda list-versions-by-function --function-name my-function
```

### Publish Version
```bash
aws lambda publish-version --function-name my-function
```

### List Aliases
```bash
aws lambda list-aliases --function-name my-function
```

---

## Workflow

1. Check AWS credentials exist
2. Export credentials as environment variables
3. Perform requested Lambda operations
4. Always confirm destructive actions (delete, update code) with the user
5. Report results clearly with function ARNs and status
