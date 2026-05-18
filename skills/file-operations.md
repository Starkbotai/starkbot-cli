---
description: "Read, write, and edit files with best practices"
version: 1.0.0
tags: [development, filesystem, editing]
requires_tools: [read_file, write_file, edit_file, list_files, find_files]
---

# File Operations

## Reading Files

### Read a File
```
read_file(path: "src/main.rs")
```
Returns the full contents of the file. Use this before any edit to understand the current state.

### List Files in a Directory
```
list_files(path: "src/")
```
Returns file and directory names at the given path. Does not recurse.

### Find Files by Pattern
```
find_files(pattern: "*.rs", path: "src/")
```
Recursively searches for files matching a glob pattern.

### Search File Contents
```
grep(pattern: "fn main", path: "src/")
```
Searches for a regex pattern across files. Returns matching lines with file paths and line numbers.

## Writing Files

### Create a New File
```
write_file(path: "src/new_module.rs", content: "// New module\n\npub fn hello() {\n    println!(\"Hello!\");\n}")
```
Creates or overwrites the file at the given path. **Always confirm with the user before overwriting an existing file.**

### Best Practices for Writing
- Read the file first if it already exists
- Match the existing code style (indentation, naming conventions, line endings)
- Include necessary imports/headers
- Don't write files with trailing whitespace

## Editing Files

### Edit a Section of a File
```
edit_file(path: "src/main.rs", old_content: "fn old_function() {\n    // old code\n}", new_content: "fn new_function() {\n    // new code\n}")
```
Replaces `old_content` with `new_content` in the file. The `old_content` must exactly match a section of the file (including whitespace and indentation).

### Best Practices for Editing
- Always `read_file` first to see the current contents
- Copy the exact text for `old_content` — whitespace matters
- Keep edits minimal: only change what's necessary
- For large rewrites, use `write_file` instead

## Common Workflows

### Add Code to an Existing File
1. `read_file` to see current contents
2. `edit_file` to insert or replace a section
3. `read_file` again to verify the result

### Create a New Module
1. `list_files` to understand the directory structure
2. `write_file` to create the new file
3. `edit_file` on the parent module to add `mod new_module;`

### Rename or Move Content
1. `read_file` on the source
2. `edit_file` to remove from the source (or delete the file)
3. `write_file` or `edit_file` to add to the destination
4. `grep` to find and update all references

### Search and Replace Across Files
1. `grep` to find all occurrences
2. `edit_file` on each file to make the replacement
3. `grep` again to verify no occurrences remain

## Tips
- Prefer `edit_file` over `write_file` for changes to existing files — it's safer and shows intent
- When making multiple edits to the same file, read once, then apply edits sequentially
- For binary files or files you don't understand, ask the user before modifying
- Always verify destructive operations (deleting content, overwriting files) with the user
