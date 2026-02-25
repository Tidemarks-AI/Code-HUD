use crate::error::CodehudError;
use crate::extractor::Item;
use crate::languages::detect_language;
use serde::Serialize;
use serde_json;
use std::path::Path;

#[derive(Serialize)]
struct JsonOutput {
    files: Vec<FileOutput>,
}

#[derive(Serialize)]
struct FileOutput {
    path: String,
    items: Vec<JsonItem>,
}

#[derive(Serialize)]
struct JsonItem {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    visibility: String,
    line_start: usize,
    line_end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    content: String,
}

/// Format list-symbols output as JSON: array of {kind, name, line} per file
pub fn format_list_symbols(files: &[(String, Vec<Item>)]) -> Result<String, CodehudError> {
    #[derive(Serialize)]
    struct SymbolEntry {
        kind: String,
        name: String,
        line: usize,
        line_end: usize,
        visibility: String,
    }

    #[derive(Serialize)]
    struct FileSymbols {
        path: String,
        symbols: Vec<SymbolEntry>,
    }

    let result: Vec<FileSymbols> = files
        .iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(path, items)| {
            let lang = detect_language(Path::new(path))
                .ok()
                .or_else(|| {
                    crate::sfc::detect_sfc(Path::new(path))
                        .map(|_| crate::languages::Language::TypeScript)
                });
            let symbols = items
                .iter()
                .map(|item| {
                    let kind = match lang {
                        Some(l) => item.kind.display_name(l).to_string(),
                        None => format!("{:?}", item.kind).to_lowercase(),
                    };
                    SymbolEntry {
                        kind,
                        name: item.name.clone().unwrap_or_else(|| "-".to_string()),
                        line: item.line_start,
                        line_end: item.line_end,
                        visibility: format!("{:?}", item.visibility).to_lowercase(),
                    }
                })
                .collect();
            FileSymbols {
                path: path.clone(),
                symbols,
            }
        })
        .collect();

    Ok(serde_json::to_string_pretty(&result)?)
}

/// Format items as JSON
pub fn format_output(files: &[(String, Vec<Item>)]) -> Result<String, CodehudError> {
    let files_output: Vec<FileOutput> = files
        .iter()
        .map(|(path, items)| {
            let lang = detect_language(Path::new(path))
                .ok()
                .or_else(|| {
                    crate::sfc::detect_sfc(Path::new(path))
                        .map(|_| crate::languages::Language::TypeScript)
                });
            let json_items: Vec<JsonItem> = items
                .iter()
                .map(|item| JsonItem {
                    kind: match lang {
                        Some(l) => item.kind.display_name(l).to_string(),
                        None => format!("{:?}", item.kind).to_lowercase(),
                    },
                    name: item.name.clone(),
                    visibility: format!("{:?}", item.visibility).to_lowercase(),
                    line_start: item.line_start,
                    line_end: item.line_end,
                    signature: item.signature.clone(),
                    body: item.body.clone(),
                    content: item.content.clone(),
                })
                .collect();

            FileOutput {
                path: path.clone(),
                items: json_items,
            }
        })
        .collect();

    let output = JsonOutput {
        files: files_output,
    };

    Ok(serde_json::to_string_pretty(&output)?)
}
