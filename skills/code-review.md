---
description: Code quality review checklist and methodology
version: 1.0.0
tags: [methodology, development]
requires_tools: [read_file, grep]
---

# Code Review Methodology

## Correctness
- Does the code do what it claims to do?
- Are error cases handled?
- Are there off-by-one errors, null pointer risks, or race conditions?

## Security
- Is user input validated?
- Are there injection risks (SQL, command, XSS)?
- Are secrets properly handled?

## Performance
- Are there unnecessary allocations or copies?
- Are database queries efficient?
- Are there N+1 query patterns?

## Maintainability
- Is the code readable without comments?
- Are functions small and focused?
- Is there unnecessary complexity?

## Testing
- Are critical paths tested?
- Are edge cases covered?
- Do tests actually assert the right things?
