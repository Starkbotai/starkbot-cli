use async_trait::async_trait;
use std::path::Path;

const DEFAULT_LINE_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

pub struct ReadFileTool;

#[async_trait]
impl metalcraft::Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str {
        "Read the contents of a file. By default reads up to 2000 lines from the beginning. \
         Use offset and limit for large files. Lines longer than 2000 characters are truncated."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path (absolute or relative to working directory)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-based). Defaults to 1."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read. Defaults to 2000."
                }
            },
            "required": ["path"]
        })
    }
    async fn call(&self, args: serde_json::Value) -> metalcraft::Result<serde_json::Value> {
        let path_str = args["path"].as_str().ok_or_else(|| metalcraft::GraphError::ToolCallFailed {
            tool: "read_file".into(), message: "Missing required parameter: path".into(),
        })?;

        let path = Path::new(path_str);

        if !path.exists() {
            return Err(metalcraft::GraphError::ToolCallFailed {
                tool: "read_file".into(),
                message: format!("File not found: {}", path_str),
            });
        }

        if path.is_dir() {
            return Err(metalcraft::GraphError::ToolCallFailed {
                tool: "read_file".into(),
                message: format!("{} is a directory, not a file. Use list_files to list directory contents.", path_str),
            });
        }

        let content = std::fs::read_to_string(path).map_err(|e| metalcraft::GraphError::ToolCallFailed {
            tool: "read_file".into(), message: format!("Failed to read {}: {}", path_str, e),
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let offset = args["offset"]
            .as_u64()
            .map(|n| (n.max(1) - 1) as usize)
            .unwrap_or(0);

        let limit = args["limit"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LINE_LIMIT);

        if offset >= total_lines {
            return Ok(serde_json::json!({
                "path": path_str,
                "content": "",
                "total_lines": total_lines,
                "note": format!("Offset {} exceeds file length of {} lines.", offset + 1, total_lines)
            }));
        }

        let end = (offset + limit).min(total_lines);
        let truncated_line_count = lines[offset..end]
            .iter()
            .filter(|l| l.len() > MAX_LINE_LENGTH)
            .count();

        let selected: String = lines[offset..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = offset + i + 1;
                if line.len() > MAX_LINE_LENGTH {
                    format!("{:>6}\t{}...", line_num, &line[..MAX_LINE_LENGTH])
                } else {
                    format!("{:>6}\t{}", line_num, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let lines_shown = end - offset;
        let remaining = total_lines - end;

        let mut result = serde_json::json!({
            "path": path_str,
            "content": selected,
            "lines_shown": lines_shown,
            "total_lines": total_lines,
        });

        if remaining > 0 {
            result["remaining_lines"] = serde_json::json!(remaining);
            result["note"] = serde_json::json!(format!(
                "{} more lines not shown. Use offset={} to continue reading.",
                remaining, end + 1
            ));
        }

        if truncated_line_count > 0 {
            result["truncated_lines"] = serde_json::json!(truncated_line_count);
        }

        Ok(result)
    }
}
