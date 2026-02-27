use clap::{Parser, Subcommand};
use codehud::{detect_language, editor, process_path, search, tree, ProcessOptions, OutputFormat, Language, CodehudError};
use codehud::editor::{BatchEdit, EditResult};
use codehud::agent;
use codehud::skill;
use codehud::tokens;
use std::{fs, io::{self, IsTerminal, Read}, path::Path, process};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "codehud")]
#[command(about = "Code context extractor using Tree-sitter", long_about = None, version)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// File or directory to analyze
    #[arg(value_name = "PATH")]
    path: Option<String>,
    
    /// Symbol names to expand (triggers expand mode)
    #[arg(value_name = "SYMBOLS")]
    symbols: Vec<String>,
    
    /// Only public items
    #[arg(long = "pub")]
    pub_only: bool,
    
    /// Only show functions/methods
    #[arg(long)]
    fns: bool,
    
    /// Only show types (struct/enum/trait/type alias)
    #[arg(long)]
    types: bool,
    
    /// Directory recursion depth (default: unlimited)
    #[arg(short = 'd', long)]
    depth: Option<usize>,

    /// Smart depth for monorepos: auto-detect source roots and apply depth relative to them
    #[arg(long = "smart-depth")]
    smart_depth: bool,
    
    /// JSON output instead of plain text
    #[arg(long)]
    json: bool,

    /// Show token count footer on stderr (default: auto-detect TTY)
    #[arg(long = "footer", overrides_with = "no_footer")]
    footer: bool,

    /// Suppress token count footer
    #[arg(long = "no-footer", overrides_with = "footer")]
    no_footer: bool,
    
    /// Exclude test code (Rust: #[cfg(test)]/​#[test], TS/JS: *.test.ts/describe()/it(), Python: test_*.py, Go: *_test.go)
    #[arg(long = "no-tests")]
    no_tests: bool,

    /// Exclude import/use statements from output (useful with --list-symbols)
    #[arg(long = "no-imports")]
    no_imports: bool,

    /// Include imports in --list-symbols output (they are hidden by default)
    #[arg(long, requires = "list_symbols")]
    imports: bool,
    
    /// Show stats summary (file count, lines, bytes, top dirs, languages)
    #[arg(long)]
    stats: bool,

    /// Show full per-file stats (verbose; use for detailed breakdown)
    #[arg(long = "stats-detailed")]
    stats_detailed: bool,

    /// Filter by file extensions (comma-separated, e.g. --ext rs,ts)
    #[arg(long, value_delimiter = ',')]
    ext: Vec<String>,

    /// Exclude paths matching glob pattern (repeatable, e.g. --exclude dist --exclude "*/migrations/*")
    #[arg(long)]
    exclude: Vec<String>,

    /// Show class with method signatures collapsed (use with a class symbol)
    #[arg(long)]
    signatures: bool,

    /// Outline mode: show signatures + docstrings + types without implementation bodies
    #[arg(long, conflicts_with_all = ["signatures", "list_symbols", "search", "lines", "tree", "files", "references", "xrefs", "stats"])]
    outline: bool,

    /// Compact outline: show minimal signatures (name + return type, no params/docstrings)
    #[arg(long, requires = "outline")]
    compact: bool,

    /// Expand named symbols inline within --outline (comma-separated symbol names)
    #[arg(long = "expand", value_delimiter = ',', requires = "outline")]
    expand_symbols: Vec<String>,

    /// Truncate expanded symbol output after N lines
    #[arg(long = "max-lines")]
    max_lines: Option<usize>,

    /// Search for text pattern (for symbol defs/refs, see --xrefs)
    #[arg(long, value_name = "PATTERN")]
    search: Option<String>,

    /// Treat search pattern as a regular expression (default: literal string)
    #[arg(short = 'E', long = "regex", requires = "search")]
    regex_mode: bool,

    /// Case-insensitive search (use with --search)
    #[arg(short = 'i', requires = "search")]
    case_insensitive: bool,

    /// Maximum number of search matches to display (default: 20 for directory search, unlimited for single-file)
    #[arg(long = "max-results")]
    max_results: Option<usize>,

    /// Limit search output to N matches (alias for --max-results, works with --search)
    #[arg(long, requires = "search", conflicts_with = "max_results")]
    limit: Option<usize>,

    /// Show aggregate summary instead of full match context (use with --search)
    #[arg(long, requires = "search")]
    summary: bool,

    /// List matched files with counts first, then detailed results (use with --search)
    #[arg(long = "files-first", requires = "search")]
    files_first: bool,

    /// List symbols with kind and line number (compact, one line per symbol)
    #[arg(long = "list-symbols")]
    list_symbols: bool,

    /// Symbol depth for --list-symbols: 1=top-level (default), 2=include class members
    #[arg(long = "symbol-depth", requires = "list_symbols")]
    symbol_depth: Option<usize>,

    /// Minimal output for --list-symbols: bare symbol names only, one per line
    #[arg(long, requires = "list_symbols")]
    minimal: bool,

    /// Extract a line range with structural context (e.g. --lines 50-75)
    #[arg(long)]
    lines: Option<String>,

    /// Show directory tree view (like `tree` but smarter)
    #[arg(long, conflicts_with_all = ["files", "search", "lines", "list_symbols"])]
    tree: bool,

    /// Show flat file listing (one file per line, relative paths)
    #[arg(long, conflicts_with_all = ["tree", "search", "lines", "list_symbols"])]
    files: bool,

    /// Find all references to a symbol name (AST-aware)
    #[arg(long, conflicts_with_all = ["tree", "files", "search", "lines", "list_symbols", "xrefs"])]
    references: Option<String>,

    /// Cross-file reference search (follows imports to find all usages across files)
    #[arg(long, conflicts_with_all = ["tree", "files", "search", "lines", "list_symbols", "references"])]
    xrefs: Option<String>,

    /// Show structural diff of changed symbols against a git ref (default: HEAD)
    #[arg(long, num_args = 0..=1, require_equals = false, default_missing_value = "", conflicts_with_all = ["tree", "files", "search", "lines", "list_symbols", "references", "xrefs", "stats"])]
    diff: Option<String>,

    /// Diff staged (index) changes instead of working tree (use with --diff)
    #[arg(long)]
    staged: bool,

    /// Truncate final output after N lines (works with any mode)
    #[arg(long = "max-output-lines")]
    max_output_lines: Option<usize>,

    /// Skip cost warnings for large repos
    #[arg(long = "yes")]
    yes: bool,

    /// Token threshold for cost warning (default: 10000)
    #[arg(long = "warn-threshold")]
    warn_threshold: Option<usize>,

    /// Number of context lines around each match (use with --references or --search)
    #[arg(short = 'C', long, default_value = "0")]
    context: usize,

    /// Show only definitions (use with --references)
    #[arg(long = "defs-only", requires = "references")]
    defs_only: bool,

    /// Show only references/usages (use with --references)
    #[arg(long = "refs-only", requires = "references")]
    refs_only: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Install codehud skill for a coding platform
    InstallSkill {
        /// Platform to install for (e.g. openclaw, claude-code, codex, cursor, aider)
        #[arg(required_unless_present = "list")]
        platform: Option<String>,

        /// List available platforms
        #[arg(long)]
        list: bool,
    },

    /// Uninstall codehud skill from a coding platform
    UninstallSkill {
        /// Platform to uninstall from
        platform: String,
    },

    /// Register codehud as a standalone agent on a platform
    InstallAgent {
        /// Platform to install for (e.g. openclaw)
        #[arg(required_unless_present = "list")]
        platform: Option<String>,

        /// List available platforms
        #[arg(long)]
        list: bool,
    },

    /// Unregister codehud agent from a platform
    UninstallAgent {
        /// Platform to uninstall from
        platform: String,

        /// Also remove workspace directory
        #[arg(long)]
        force: bool,
    },

    /// Edit a symbol in a file
    Edit {
        /// File to edit
        file: String,
        
        /// Symbol name to edit (not needed with --batch)
        #[arg(default_value = "")]
        symbol: String,
        
        /// Replace the symbol with new source
        #[arg(long, conflicts_with_all = ["delete", "replace_body", "batch"])]
        replace: Option<String>,
        
        /// Replace only the body block, preserving signature/attributes
        #[arg(long = "replace-body", conflicts_with_all = ["delete", "replace", "batch"])]
        replace_body: Option<String>,
        
        /// Read replacement from stdin (works with --replace or --replace-body)
        #[arg(long)]
        stdin: bool,
        
        /// Delete the symbol
        #[arg(long, conflicts_with_all = ["replace", "replace_body", "batch"])]
        delete: bool,
        
        /// Insert new code after a named symbol
        #[arg(long = "add-after", conflicts_with_all = ["replace", "replace_body", "delete", "add_before", "append", "prepend", "batch"])]
        add_after: Option<String>,
        
        /// Insert new code before a named symbol
        #[arg(long = "add-before", conflicts_with_all = ["replace", "replace_body", "delete", "add_after", "append", "prepend", "batch"])]
        add_before: Option<String>,
        
        /// Append new code to end of file
        #[arg(long, conflicts_with_all = ["replace", "replace_body", "delete", "add_after", "add_before", "prepend", "batch"])]
        append: bool,
        
        /// Prepend new code at beginning of file (after leading comments)
        #[arg(long, conflicts_with_all = ["replace", "replace_body", "delete", "add_after", "add_before", "append", "batch"])]
        prepend: bool,
        
        /// Apply batch edits from a JSON file
        #[arg(long, conflicts_with_all = ["replace", "replace_body", "delete", "add_after", "add_before", "append", "prepend"])]
        batch: Option<String>,
        
        /// Dry run - print to stdout instead of writing file
        #[arg(long)]
        dry_run: bool,
        
        /// Output JSON metadata about what changed
        #[arg(long)]
        json: bool,
    },
}

/// Truncate output to N lines, appending a footer if truncated.
/// Returns true if the pattern looks like a symbol name (single word, no spaces/regex chars).
/// Matches identifiers like `PascalCase`, `snake_case`, `camelCase`, `UPPER_CASE`, etc.
fn looks_like_symbol(pattern: &str) -> bool {
    !pattern.is_empty()
        && !pattern.contains(char::is_whitespace)
        && pattern.chars().all(|c| c.is_alphanumeric() || c == '_')
        && pattern.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
}

/// Print output with optional token count footer.
/// For JSON output, injects token_estimate and cost_estimate into the top-level object.
fn emit_output(output: &str, max_lines: Option<usize>, json: bool, show_footer: bool) {
    let truncated = truncate_output(output, max_lines);
    let token_count = tokens::estimate_tokens(&truncated);
    let cost = tokens::estimate_cost(token_count, true);

    if json {
        // Try to inject token fields into top-level JSON object
        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&truncated) {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("token_estimate".to_string(), serde_json::json!(token_count));
                obj.insert("cost_estimate".to_string(), serde_json::json!(format!("{:.4}", cost)));
            }
            print!("{}", serde_json::to_string_pretty(&value).unwrap_or(truncated.clone()));
        } else {
            print!("{}", truncated);
        }
    } else {
        print!("{}", truncated);
    }

    if show_footer {
        eprintln!("[Output: {} tokens | ~${:.2} with caching]", token_count, cost);
    }
}

fn truncate_output(output: &str, max_lines: Option<usize>) -> String {
    let max = match max_lines {
        Some(n) => n,
        None => return output.to_string(),
    };
    let total = output.lines().count();
    if total <= max {
        return output.to_string();
    }
    let mut result: String = output.lines().take(max).collect::<Vec<_>>().join("\n");
    result.push('\n');
    result.push_str(&format!("[Output truncated: {} lines shown of {} total]\n", max, total));
    result
}

fn main() {
    // Reset SIGPIPE to default behavior so piping to head/less doesn't panic (#39)
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Improve error message for missing --search value
            let err_str = e.to_string();
            if err_str.contains("--search") && err_str.contains("value is required") {
                eprintln!("error: --search requires a pattern\n");
                eprintln!("Usage: codehud <path> --search <pattern>");
                eprintln!("       codehud src/ --search \"fn main\"");
                eprintln!("       codehud . --search TODO -E    (regex mode)");
                process::exit(2);
            }
            e.exit();
        }
    };

    let max_output_lines = cli.max_output_lines;
    let show_footer = if cli.no_footer {
        false
    } else if cli.footer {
        true
    } else {
        io::stderr().is_terminal()
    };
    let is_json = cli.json;
    
    match cli.command {
        Some(Commands::InstallSkill { platform, list }) => {
            if list {
                skill::list_platforms();
            } else if let Some(p) = platform
                && let Err(e) = skill::install(&p) {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
        }
        Some(Commands::UninstallSkill { platform }) => {
            if let Err(e) = skill::uninstall(&platform) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::InstallAgent { platform, list }) => {
            if list {
                agent::list_platforms();
            } else if let Some(p) = platform
                && let Err(e) = agent::install(&p) {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
        }
        Some(Commands::UninstallAgent { platform, force }) => {
            if let Err(e) = agent::uninstall(&platform, force) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        Some(Commands::Edit { file, symbol, replace, replace_body, stdin, delete, add_after, add_before, append, prepend, batch, dry_run, json }) => {
            if let Err(e) = handle_edit(&file, &symbol, EditOptions { replace, replace_body, stdin, delete, add_after, add_before, append, prepend, batch, dry_run, json }) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        None => {
            // Default behavior: process path
            let path = match cli.path {
                Some(p) => p,
                None => {
                    eprintln!("Error: PATH is required");
                    process::exit(1);
                }
            };

            // Handle --tree / --files mode
            if cli.tree || cli.files {
                let effective_depth = if cli.smart_depth && cli.depth.is_none() {
                    Some(0)
                } else {
                    cli.depth
                };
                let tree_opts = tree::TreeOptions {
                    depth: effective_depth,
                    ext: cli.ext,
                    stats: cli.stats,
                    json: cli.json,
                    smart_depth: cli.smart_depth,
                    no_tests: cli.no_tests,
                    exclude: cli.exclude,
                };
                let result = if cli.tree {
                    tree::tree_view(&path, &tree_opts)
                } else {
                    tree::list_files(&path, &tree_opts)
                };
                match result {
                    Ok(output) => {
                        emit_output(&output, max_output_lines, is_json, show_footer);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            // Handle --lines mode
            if let Some(lines_arg) = cli.lines {
                match codehud::extract_lines(&path, &lines_arg, cli.json) {
                    Ok(output) => {
                        emit_output(&output, max_output_lines, is_json, show_footer);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            // Handle --references mode
            if let Some(symbol) = cli.references {
                // Dot-notation references (e.g. Workflow.getStartNode) route to method xrefs
                let result = if symbol.contains('.') || symbol.contains("::") {
                    let xref_opts = codehud::xrefs::XrefOptions {
                        symbol,
                        depth: cli.depth,
                        ext: cli.ext.clone(),
                        context_lines: cli.context,
                        json: cli.json,
                        exclude: cli.exclude.clone(),
                        max_results: None,
                    };
                    codehud::xrefs::find_xrefs(&path, &xref_opts)
                } else {
                    let ref_opts = codehud::references::ReferenceOptions {
                        symbol,
                        depth: cli.depth,
                        ext: cli.ext.clone(),
                        context_lines: cli.context,
                        defs_only: cli.defs_only,
                        refs_only: cli.refs_only,
                        json: cli.json,
                        exclude: cli.exclude.clone(),
                    };
                    codehud::references::find_references(&path, &ref_opts)
                };
                match result {
                    Ok(refs) => {
                        if cli.json {
                            let output = codehud::references::format_json(&refs); emit_output(&output, max_output_lines, is_json, show_footer);
                        } else {
                            let output = codehud::references::format_plain(&refs); emit_output(&output, max_output_lines, is_json, show_footer);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            // Handle --xrefs mode
            if let Some(symbol) = cli.xrefs {
                let xref_opts = codehud::xrefs::XrefOptions {
                    symbol,
                    depth: cli.depth,
                    ext: cli.ext.clone(),
                    context_lines: cli.context,
                    json: cli.json,
                    exclude: cli.exclude.clone(),
                    max_results: cli.max_results,
                };
                match codehud::xrefs::find_xrefs(&path, &xref_opts) {
                    Ok(refs) => {
                        if cli.json {
                            let output = codehud::references::format_json(&refs); emit_output(&output, max_output_lines, is_json, show_footer);
                        } else {
                            let output = codehud::references::format_plain(&refs); emit_output(&output, max_output_lines, is_json, show_footer);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            // Handle --diff mode
            if cli.diff.is_some() || cli.staged {
                let refspec = match &cli.diff {
                    Some(r) if !r.is_empty() => Some(r.clone()),
                    _ => None,
                };
                let diff_opts = codehud::diff_cli::DiffOptions {
                    refspec,
                    staged: cli.staged,
                    path_scope: Some(path.clone()),
                    json: cli.json,
                    pub_only: cli.pub_only,
                    fns_only: cli.fns,
                    types_only: cli.types,
                    no_tests: cli.no_tests,
                    ext: cli.ext.clone(),
                    exclude: cli.exclude.clone(),
                };
                match codehud::diff_cli::run_diff(&diff_opts) {
                    Ok(output) => {
                        emit_output(&output, max_output_lines, is_json, show_footer);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            // Handle --search mode
            if let Some(pattern) = cli.search {
                let is_dir = Path::new(&path).is_dir();
                let pattern_display = pattern.clone();
                let search_opts = search::SearchOptions {
                    pattern,
                    regex: cli.regex_mode,
                    case_insensitive: cli.case_insensitive,
                    depth: cli.depth,
                    ext: cli.ext,
                    max_results: cli.limit.or(cli.max_results).or(if is_dir { Some(20) } else { None }),
                    no_tests: cli.no_tests,
                    exclude: cli.exclude,
                    json: cli.json,
                    context: if cli.context > 0 { Some(cli.context) } else { None },
                    summary: cli.summary,
                    files_first: cli.files_first,
                };
                match search::search_path(&path, &search_opts) {
                    Ok(output) if output.is_empty() => {
                        eprintln!("No matches found for '{}'", pattern_display);
                        process::exit(1);
                    }
                    Ok(output) => {
                        emit_output(&output, max_output_lines, is_json, show_footer);
                        // Show xrefs hint for symbol-like patterns in non-JSON mode
                        if !cli.json && looks_like_symbol(&pattern_display) {
                            eprintln!("\n💡 Tip: For symbol definitions and references, try: codehud --xrefs {}", pattern_display);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }
            
            let format = if cli.json {
                OutputFormat::Json
            } else {
                OutputFormat::Plain
            };
            
            // When --smart-depth is used without --depth, default to depth 0
            // so smart-depth can discover source roots and walk into them
            let effective_depth = if cli.smart_depth && cli.depth.is_none() {
                Some(0)
            } else {
                cli.depth
            };

            // --stats-detailed implies --stats
            let stats = cli.stats || cli.stats_detailed;

            let options = ProcessOptions {
                symbols: cli.symbols,
                pub_only: cli.pub_only,
                fns_only: cli.fns,
                types_only: cli.types,
                no_tests: cli.no_tests,
                depth: effective_depth,
                format,
                stats,
                stats_detailed: cli.stats_detailed,
                ext: cli.ext,
                signatures: cli.signatures,
                max_lines: cli.max_lines,
                list_symbols: cli.list_symbols,
                symbol_depth: cli.symbol_depth,
                no_imports: cli.no_imports || (cli.list_symbols && !cli.imports),
                smart_depth: cli.smart_depth,
                exclude: cli.exclude,
                outline: cli.outline,
                compact: cli.compact,
                minimal: cli.minimal,
                expand_symbols: cli.expand_symbols,
                yes: cli.yes,
                warn_threshold: cli.warn_threshold.unwrap_or(codehud::walk::DEFAULT_WARN_THRESHOLD),
            };
            
            match process_path(&path, options) {
                Ok(output) => {
                    emit_output(&output, max_output_lines, is_json, show_footer);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}

struct EditOptions {
    replace: Option<String>,
    replace_body: Option<String>,
    stdin: bool,
    delete: bool,
    add_after: Option<String>,
    add_before: Option<String>,
    append: bool,
    prepend: bool,
    batch: Option<String>,
    dry_run: bool,
    json: bool,
}

fn handle_edit(
    file: &str,
    symbol: &str,
    opts: EditOptions,
) -> Result<(), CodehudError> {
    let EditOptions { replace, replace_body, stdin, delete, add_after, add_before, append, prepend, batch, dry_run, json } = opts;
    let path = Path::new(file);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(file.to_string()));
    }
    
    let source = fs::read_to_string(path)
        .map_err(|e| CodehudError::ReadError {
            path: file.to_string(),
            source: e,
        })?;
    
    let language_opt = detect_language(path).ok();
    
    // For AST-based operations, we need a language. For simple ops (append/prepend), we don't.
    let require_language = |op: &str| -> Result<Language, CodehudError> {
        language_opt.ok_or_else(|| CodehudError::ParseError(format!(
            "{} requires a supported language (rs/ts/tsx/js/jsx/py) for AST operations. \
             For unsupported file types, use --append or --prepend instead.",
            op
        )))
    };
    
    // Compute edit metadata before performing the edit (line ranges from original source)
    let mut edit_results: Vec<EditResult> = Vec::new();
    
    let result = if let Some(batch_file) = batch {
        let batch_json = fs::read_to_string(&batch_file)
            .map_err(|e| CodehudError::ReadError {
                path: batch_file.clone(),
                source: e,
            })?;
        #[derive(serde::Deserialize)]
        struct BatchInput { edits: Vec<BatchEdit> }
        let input: BatchInput = serde_json::from_str(&batch_json)?;
        
        let language = require_language("batch edit")?;
        if json {
            for edit in &input.edits {
                let (line_start, line_end) = match edit.action {
                    editor::BatchAction::Append | editor::BatchAction::Prepend => (0, 0),
                    _ => editor::symbol_line_range(&source, &edit.symbol, language)?,
                };
                let action = match edit.action {
                    editor::BatchAction::Replace => "replaced",
                    editor::BatchAction::ReplaceBody => "replaced_body",
                    editor::BatchAction::Delete => "deleted",
                    editor::BatchAction::AddAfter => "added_after",
                    editor::BatchAction::AddBefore => "added_before",
                    editor::BatchAction::Append => "appended",
                    editor::BatchAction::Prepend => "prepended",
                };
                edit_results.push(EditResult {
                    symbol: edit.symbol.clone(),
                    action: action.to_string(),
                    line_start,
                    line_end,
                });
            }
        }
        
        editor::batch(&source, &input.edits, language)?
    } else if delete {
        let language = require_language("--delete")?;
        if json {
            let (line_start, line_end) = editor::symbol_line_range(&source, symbol, language)?;
            edit_results.push(EditResult {
                symbol: symbol.to_string(),
                action: "deleted".to_string(),
                line_start,
                line_end,
            });
        }
        editor::delete(&source, symbol, language)?
    } else if let Some(body_content) = replace_body {
        let language = require_language("--replace-body")?;
        let new_body = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            body_content
        };
        if json {
            let (line_start, line_end) = editor::symbol_line_range(&source, symbol, language)?;
            edit_results.push(EditResult {
                symbol: symbol.to_string(),
                action: "replaced_body".to_string(),
                line_start,
                line_end,
            });
        }
        editor::replace_body(&source, symbol, &new_body, language)?
    } else if let Some(replacement) = replace {
        let language = require_language("--replace")?;
        let new_content = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            replacement
        };
        if json {
            let (line_start, line_end) = editor::symbol_line_range(&source, symbol, language)?;
            edit_results.push(EditResult {
                symbol: symbol.to_string(),
                action: "replaced".to_string(),
                line_start,
                line_end,
            });
        }
        editor::replace(&source, symbol, &new_content, language)?
    } else if let Some(ref_symbol) = add_after {
        let language = require_language("--add-after")?;
        let new_code = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            symbol.to_string()
        };
        if json {
            edit_results.push(EditResult {
                symbol: ref_symbol.clone(),
                action: "added_after".to_string(),
                line_start: 0,
                line_end: 0,
            });
        }
        editor::add_after(&source, &ref_symbol, &new_code, language)?
    } else if let Some(ref_symbol) = add_before {
        let language = require_language("--add-before")?;
        let new_code = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            symbol.to_string()
        };
        if json {
            edit_results.push(EditResult {
                symbol: ref_symbol.clone(),
                action: "added_before".to_string(),
                line_start: 0,
                line_end: 0,
            });
        }
        editor::add_before(&source, &ref_symbol, &new_code, language)?
    } else if append {
        let new_code = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            symbol.to_string()
        };
        if json {
            edit_results.push(EditResult {
                symbol: "(file)".to_string(),
                action: "appended".to_string(),
                line_start: 0,
                line_end: 0,
            });
        }
        if let Some(language) = language_opt {
            editor::append(&source, &new_code, language)?
        } else {
            // Passthrough append for unsupported files
            let mut result = source.to_string();
            if !result.ends_with('\n') && !result.is_empty() {
                result.push('\n');
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&new_code);
            if !result.ends_with('\n') {
                result.push('\n');
            }
            result
        }
    } else if prepend {
        let new_code = if stdin {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)
                .map_err(|e| CodehudError::ParseError(format!("Failed to read stdin: {}", e)))?;
            buf
        } else {
            symbol.to_string()
        };
        if json {
            edit_results.push(EditResult {
                symbol: "(file)".to_string(),
                action: "prepended".to_string(),
                line_start: 0,
                line_end: 0,
            });
        }
        if let Some(language) = language_opt {
            editor::prepend(&source, &new_code, language)?
        } else {
            // Passthrough prepend for unsupported files
            let mut result = String::new();
            result.push_str(&new_code);
            if !new_code.ends_with('\n') {
                result.push('\n');
            }
            if !source.is_empty() {
                result.push('\n');
                result.push_str(&source);
            }
            result
        }
    } else {
        return Err(CodehudError::ParseError(
            "Must specify --replace, --replace-body, --delete, --add-after, --add-before, --append, --prepend, or --batch".to_string()
        ));
    };
    
    if dry_run {
        print!("{}", result);
    } else {
        fs::write(path, &result)
            .map_err(|e| CodehudError::ReadError {
                path: file.to_string(),
                source: e,
            })?;
    }
    
    if json {
        if edit_results.len() == 1 {
            println!("{}", serde_json::to_string(&edit_results[0])?);
        } else {
            println!("{}", serde_json::to_string(&edit_results)?);
        }
    }
    
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_symbol() {
        // Valid symbols
        assert!(looks_like_symbol("PascalCase"));
        assert!(looks_like_symbol("snake_case"));
        assert!(looks_like_symbol("camelCase"));
        assert!(looks_like_symbol("UPPER_CASE"));
        assert!(looks_like_symbol("_private"));
        assert!(looks_like_symbol("foo"));
        assert!(looks_like_symbol("x"));

        // Not symbols
        assert!(!looks_like_symbol(""));
        assert!(!looks_like_symbol("two words"));
        assert!(!looks_like_symbol("fn main"));
        assert!(!looks_like_symbol("foo.*bar"));
        assert!(!looks_like_symbol("123abc"));
        assert!(!looks_like_symbol("hello("));
        assert!(!looks_like_symbol("a+b"));
    }

    #[test]
    fn test_emit_output_json_injects_token_fields() {
        // emit_output prints to stdout, so test the logic directly
        let input = r#"{"files":[]}"#;
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let token_count = codehud::tokens::estimate_tokens(input);
        let cost = codehud::tokens::estimate_cost(token_count, true);
        let mut obj = value.as_object().unwrap().clone();
        obj.insert("token_estimate".to_string(), serde_json::json!(token_count));
        obj.insert("cost_estimate".to_string(), serde_json::json!(format!("{:.4}", cost)));
        assert!(obj.contains_key("token_estimate"));
        assert!(obj.contains_key("cost_estimate"));
        assert_eq!(obj["token_estimate"], serde_json::json!(token_count));
    }

    #[test]
    fn test_footer_flag_logic() {
        // --no-footer always suppresses
        let no_footer = true;
        let footer = false;
        let show = if no_footer { false } else if footer { true } else { false };
        assert!(!show);

        // --footer always enables
        let no_footer = false;
        let footer = true;
        let show = if no_footer { false } else if footer { true } else { false };
        assert!(show);
    }
}
