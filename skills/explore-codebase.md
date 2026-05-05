---
description: Systematic codebase exploration and understanding
version: 1.0.0
tags: [methodology, research]
requires_tools: [read_file, list_files, find_files, grep]
---

# Codebase Exploration

## Step 1: Top-Level Structure
- `list_files` at the root to see project layout
- Read README, Cargo.toml/package.json for project overview
- Identify the main entry point

## Step 2: Architecture
- Read the main entry file to understand the application flow
- Identify key modules and their responsibilities
- Look for configuration files that reveal system design

## Step 3: Key Patterns
- Search for common patterns (traits, interfaces, base classes)
- Identify how dependency injection or configuration works
- Look at how errors are handled

## Step 4: Specific Area
- Once you understand the high level, dive into the specific area of interest
- Read tests to understand expected behavior
- Follow the call chain from entry point to the area of interest

## Tips
- Start broad, then narrow down
- Read tests before implementation - they document intent
- Look at recent git history for context on why things are the way they are
