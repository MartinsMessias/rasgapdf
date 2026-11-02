use crate::spatial::{parse_file, parse_raw_file};
use crate::types::{BoundingBox, ParserOptions, SpatialChunk};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

/// Runs the Model Context Protocol (MCP) stdio JSON-RPC 2.0 server.
pub fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin);
    let mut writer = io::BufWriter::new(stdout);

    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        if let Ok(request) = serde_json::from_str::<Value>(trimmed) {
            if let Some(response) = handle_mcp_message(&request) {
                serde_json::to_writer(&mut writer, &response)?;
                writer.write_all(b"\n")?;
                writer.flush()?;
            }
        }

        line.clear();
    }

    Ok(())
}

fn handle_mcp_message(msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(|m| m.as_str())?;
    let id = msg.get("id");

    match method {
        "initialize" => {
            let req_id = id.cloned().unwrap_or(json!(1));
            Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "rasgapdf-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }))
        }
        "notifications/initialized" => None,
        "ping" => {
            let req_id = id.cloned().unwrap_or(json!(1));
            Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {}
            }))
        }
        "tools/list" => {
            let req_id = id.cloned().unwrap_or(json!(1));
            Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": [
                        {
                            "name": "parse_pdf",
                            "description": "Parse a PDF file into structured 2D spatial text blocks, tables, and exact bounding boxes using pure Rust.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Absolute or relative file path to the PDF" },
                                    "mode": { "type": "string", "enum": ["structured", "raw"], "description": "Parsing mode (default: structured)" },
                                    "reading_order": { "type": "string", "enum": ["natural", "raw"], "description": "Reading order: 'natural' (Recursive XY-Cut) or 'raw' (default: natural)" },
                                    "detect_tables": { "type": "boolean", "description": "Detect tables and grid layouts (default: true)" },
                                    "apply_rotation": { "type": "boolean", "description": "Normalize bboxes and coordinates according to page rotation (default: true)" },
                                    "max_pages": { "type": "integer", "description": "Optional limit on number of pages to return" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "search_document",
                            "description": "Search for terms in a PDF and return matching spans, lines, and exact bounding boxes for visual citations.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to the PDF file" },
                                    "query": { "type": "string", "description": "Search query text" },
                                    "case_sensitive": { "type": "boolean", "description": "Case-sensitive search (default: false)" },
                                    "max_results": { "type": "integer", "description": "Maximum number of citations to return (default: 20)" }
                                },
                                "required": ["path", "query"]
                            }
                        },
                        {
                            "name": "get_chunks",
                            "description": "Extract semantic spatial chunks optimized for LLM RAG with exact bounding boxes, breadcrumbs, and VLM triage flags.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to the PDF file" },
                                    "max_chars": { "type": "integer", "description": "Maximum characters per chunk (default: 1500)" },
                                    "overlap_blocks": { "type": "integer", "description": "Block overlap between contiguous chunks (default: 1)" },
                                    "apply_rotation": { "type": "boolean", "description": "Normalize bboxes to rotated viewer orientation (default: true)" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "get_triage",
                            "description": "Analyze visual complexity of all pages/blocks in a PDF and return sub-page bounding boxes flagged as is_complex_layout for selective Vision Model (VLM) inspection.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Path to the PDF file" }
                                },
                                "required": ["path"]
                            }
                        }
                    ]
                }
            }))
        }
        "tools/call" => {
            let req_id = id.cloned().unwrap_or(json!(1));
            let params = msg.get("params");
            let tool_name = params
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let arguments = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = match tool_name {
                "parse_pdf" => handle_tool_parse_pdf(&arguments),
                "search_document" => handle_tool_search(&arguments),
                "get_chunks" => handle_tool_get_chunks(&arguments),
                "get_triage" => handle_tool_get_triage(&arguments),
                _ => Err(format!("Unknown tool: {}", tool_name)),
            };

            match result {
                Ok(text_output) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": text_output
                            }
                        ],
                        "isError": false
                    }
                })),
                Err(err_msg) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Error: {}", err_msg)
                            }
                        ],
                        "isError": true
                    }
                })),
            }
        }
        _ => id.map(|req_id| {
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }),
    }
}

fn handle_tool_parse_pdf(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or("Missing 'path' argument")?;
    let mode = args
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("structured");
    let reading_order = args
        .get("reading_order")
        .and_then(|r| r.as_str())
        .unwrap_or("natural");
    let detect_tables = args
        .get("detect_tables")
        .and_then(|d| d.as_bool())
        .unwrap_or(true);
    let apply_rot = args
        .get("apply_rotation")
        .and_then(|a| a.as_bool())
        .unwrap_or(true);
    let max_pages = args
        .get("max_pages")
        .and_then(|m| m.as_u64())
        .map(|u| u as usize);

    if mode == "raw" {
        let raw_doc = parse_raw_file(path).map_err(|e| e.to_string())?;
        return serde_json::to_string_pretty(&raw_doc).map_err(|e| e.to_string());
    }

    let options = ParserOptions {
        mode: "structured".to_string(),
        reading_order: reading_order.to_string(),
        detect_tables,
        apply_rotation: apply_rot,
        ..Default::default()
    };

    let mut doc = parse_file(path, &options).map_err(|e| e.to_string())?;

    if apply_rot {
        doc.pages = doc.pages.into_iter().map(|p| p.rotated()).collect();
    }

    if let Some(limit) = max_pages {
        doc.pages.truncate(limit);
        doc.total_pages = doc.pages.len();
    }

    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct SearchResultItem {
    page: usize,
    text: String,
    bbox: BoundingBox,
    font_name: String,
    font_size: f32,
    block_id: usize,
}

fn handle_tool_search(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or("Missing 'path' argument")?;
    let query = args
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or("Missing 'query' argument")?;
    let case_sensitive = args
        .get("case_sensitive")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);
    let max_results = args
        .get("max_results")
        .and_then(|m| m.as_u64())
        .unwrap_or(20) as usize;

    let options = ParserOptions {
        reading_order: "natural".to_string(),
        apply_rotation: true,
        ..Default::default()
    };
    let doc = parse_file(path, &options).map_err(|e| e.to_string())?;

    let mut matches = Vec::new();
    let q_cmp = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    for page in &doc.pages {
        for block in &page.text_blocks {
            for line in &block.lines {
                let line_cmp = if case_sensitive {
                    line.text.clone()
                } else {
                    line.text.to_lowercase()
                };

                if line_cmp.contains(&q_cmp) {
                    for span in &line.spans {
                        let span_cmp = if case_sensitive {
                            span.text.clone()
                        } else {
                            span.text.to_lowercase()
                        };
                        if span_cmp.contains(&q_cmp) {
                            matches.push(SearchResultItem {
                                page: page.page_number,
                                text: span.text.clone(),
                                bbox: span.bbox,
                                font_name: span.font_name.clone(),
                                font_size: span.font_size,
                                block_id: block.block_id,
                            });
                            if matches.len() >= max_results {
                                break;
                            }
                        }
                    }

                    if matches.len() < max_results
                        && !matches.iter().any(|m| {
                            m.page == page.page_number
                                && m.block_id == block.block_id
                                && m.text == line.text
                        })
                    {
                        matches.push(SearchResultItem {
                            page: page.page_number,
                            text: line.text.clone(),
                            bbox: line.bbox,
                            font_name: String::new(),
                            font_size: 0.0,
                            block_id: block.block_id,
                        });
                    }

                    if matches.len() >= max_results {
                        break;
                    }
                }
            }
            if matches.len() >= max_results {
                break;
            }
        }
        if matches.len() >= max_results {
            break;
        }
    }

    serde_json::to_string_pretty(&matches).map_err(|e| e.to_string())
}

fn handle_tool_get_chunks(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or("Missing 'path' argument")?;
    let max_chars = args
        .get("max_chars")
        .and_then(|m| m.as_u64())
        .unwrap_or(1500) as usize;
    let overlap_blocks = args
        .get("overlap_blocks")
        .and_then(|o| o.as_u64())
        .unwrap_or(1) as usize;
    let apply_rot = args
        .get("apply_rotation")
        .and_then(|a| a.as_bool())
        .unwrap_or(true);

    let options = ParserOptions {
        reading_order: "natural".to_string(),
        apply_rotation: apply_rot,
        ..Default::default()
    };
    let mut doc = parse_file(path, &options).map_err(|e| e.to_string())?;

    if apply_rot {
        doc.pages = doc.pages.into_iter().map(|p| p.rotated()).collect();
    }

    let mut all_chunks: Vec<SpatialChunk> = Vec::new();
    for page in &doc.pages {
        let page_chunks = page.to_spatial_chunks(max_chars, overlap_blocks);
        all_chunks.extend(page_chunks);
    }

    serde_json::to_string_pretty(&all_chunks).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct TriageRegion {
    page_number: usize,
    block_id: usize,
    bbox: BoundingBox,
    complexity_score: f32,
    reason: String,
    preview_text: String,
}

#[derive(Serialize)]
struct TriageReport {
    file_name: String,
    total_pages: usize,
    total_complex_regions: usize,
    complex_regions: Vec<TriageRegion>,
}

fn handle_tool_get_triage(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or("Missing 'path' argument")?;
    let options = ParserOptions {
        reading_order: "natural".to_string(),
        extract_drawings: true,
        ..Default::default()
    };
    let doc = parse_file(path, &options).map_err(|e| e.to_string())?;

    let mut complex_regions = Vec::new();

    for page in &doc.pages {
        for block in &page.text_blocks {
            if block.is_complex_layout || block.complexity_score.unwrap_or(0.0) >= 0.45 {
                let score = block.complexity_score.unwrap_or(0.50);
                let preview: String = block
                    .lines
                    .iter()
                    .take(2)
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");

                complex_regions.push(TriageRegion {
                    page_number: page.page_number,
                    block_id: block.block_id,
                    bbox: block.bbox,
                    complexity_score: score,
                    reason: "visual_or_geometric_complexity".to_string(),
                    preview_text: preview,
                });
            }
        }
    }

    let report = TriageReport {
        file_name: doc.file_name.clone(),
        total_pages: doc.total_pages,
        total_complex_regions: complex_regions.len(),
        complex_regions,
    };

    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let res = handle_mcp_message(&req).unwrap();
        assert_eq!(res["result"]["serverInfo"]["name"], "rasgapdf-mcp");
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        let res = handle_mcp_message(&req).unwrap();
        let tools = res["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "parse_pdf"));
        assert!(tools.iter().any(|t| t["name"] == "search_document"));
        assert!(tools.iter().any(|t| t["name"] == "get_chunks"));
        assert!(tools.iter().any(|t| t["name"] == "get_triage"));
    }

    #[test]
    fn test_mcp_tools_call_sample_pdf() {
        if !std::path::Path::new("docs/sample.pdf").exists() {
            return;
        }

        // 1. parse_pdf
        let parse_req = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "parse_pdf",
                "arguments": { "path": "docs/sample.pdf", "mode": "structured" }
            }
        });
        let parse_res = handle_mcp_message(&parse_req).unwrap();
        assert_eq!(parse_res["result"]["isError"], false);
        let text_content = parse_res["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text_content.contains("file_name"));

        // 2. search_document
        let search_req = json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "search_document",
                "arguments": { "path": "docs/sample.pdf", "query": "RasgaPDF" }
            }
        });
        let search_res = handle_mcp_message(&search_req).unwrap();
        assert_eq!(search_res["result"]["isError"], false);

        // 3. get_chunks
        let chunks_req = json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "get_chunks",
                "arguments": { "path": "docs/sample.pdf", "max_chars": 500 }
            }
        });
        let chunks_res = handle_mcp_message(&chunks_req).unwrap();
        assert_eq!(chunks_res["result"]["isError"], false);

        // 4. get_triage
        let triage_req = json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "tools/call",
            "params": {
                "name": "get_triage",
                "arguments": { "path": "docs/sample.pdf" }
            }
        });
        let triage_res = handle_mcp_message(&triage_req).unwrap();
        assert_eq!(triage_res["result"]["isError"], false);
    }
}
