---
description: Systematic approach to debugging code issues
version: 1.0.0
tags: [methodology, development]
requires_tools: [read_file, grep, bash, edit_file]
---

# Debugging Methodology

## Step 1: Reproduce
- Identify the exact error message or unexpected behavior
- Create a minimal reproduction case if possible
- Note the exact steps to trigger the issue

## Step 2: Investigate
- Read the relevant source files around the error location
- Search for error patterns with grep
- Check recent changes: `git log --oneline -20` and `git diff`
- Look at test files for expected behavior

## Step 3: Hypothesize
- Form a theory about the root cause
- Identify the specific code path that leads to the error
- Consider edge cases and boundary conditions

## Step 4: Fix
- Apply the minimal fix that addresses the root cause
- Do not refactor or clean up unrelated code
- Preserve existing behavior for non-buggy paths

## Step 5: Verify
- Run the relevant tests: `cargo test` or equivalent
- If no tests exist for the bug, consider adding one
- Check for regressions in related functionality
