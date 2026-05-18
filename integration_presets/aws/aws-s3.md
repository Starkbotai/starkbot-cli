---
description: "Manage AWS S3 buckets and objects via the AWS CLI"
version: 1.0.0
tags: [infrastructure, cloud, aws, devops]
requires_tools: [bash, api_keys_check]
---

# AWS S3 Management

## Prerequisites

Requires API keys: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_REGION`.

Before any operation, check the keys exist:
```
api_keys_check(service_name: "AWS_ACCESS_KEY_ID")
api_keys_check(service_name: "AWS_SECRET_ACCESS_KEY")
```

If not configured, ask the user to provide them and install via `install_integration`.
Keys are created in the AWS IAM console: https://console.aws.amazon.com/iam/

## CLI Setup

AWS operations use the `aws` CLI tool via `bash`. Before running commands, export credentials:
```bash
export AWS_ACCESS_KEY_ID="<resolved from keystore>"
export AWS_SECRET_ACCESS_KEY="<resolved from keystore>"
export AWS_REGION="${AWS_REGION:-us-east-1}"
```

**IMPORTANT**: Use `api_key_read` to retrieve key values, then pass them as environment variables to `bash`. Never log or echo secret keys.

---

## Buckets

### List Buckets
```bash
aws s3 ls
```

### Create Bucket
```bash
aws s3 mb s3://bucket-name --region us-east-1
```

### Delete Bucket
```bash
aws s3 rb s3://bucket-name --force
```
`--force` removes all objects first. Always confirm with user before using.

---

## Objects

### List Objects
```bash
aws s3 ls s3://bucket-name/
aws s3 ls s3://bucket-name/prefix/ --recursive
```

### Copy Files
```bash
# Upload
aws s3 cp local-file.txt s3://bucket-name/key.txt

# Download
aws s3 cp s3://bucket-name/key.txt local-file.txt

# Copy between buckets
aws s3 cp s3://source-bucket/key.txt s3://dest-bucket/key.txt
```

### Move Files
```bash
aws s3 mv s3://bucket-name/old-key.txt s3://bucket-name/new-key.txt
```

### Delete Objects
```bash
aws s3 rm s3://bucket-name/key.txt
aws s3 rm s3://bucket-name/prefix/ --recursive
```

### Sync Directories
```bash
# Upload directory
aws s3 sync ./local-dir s3://bucket-name/prefix/

# Download directory
aws s3 sync s3://bucket-name/prefix/ ./local-dir

# With delete (mirror)
aws s3 sync ./local-dir s3://bucket-name/prefix/ --delete
```

### Generate Presigned URL
```bash
aws s3 presign s3://bucket-name/key.txt --expires-in 3600
```
Returns a URL valid for the specified number of seconds (default 3600).

---

## Bucket Info

### Get Bucket Location
```bash
aws s3api get-bucket-location --bucket bucket-name
```

### Get Bucket Size (approximate)
```bash
aws s3 ls s3://bucket-name --recursive --summarize
```

---

## Workflow

1. Check AWS credentials exist
2. Export credentials as environment variables
3. Perform requested S3 operations
4. Always confirm destructive actions (delete, rb --force, sync --delete) with the user
5. Report results clearly
