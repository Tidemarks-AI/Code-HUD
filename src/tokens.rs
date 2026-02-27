/// Estimated USD cost per token (uncached input).
const COST_PER_TOKEN_UNCACHED: f64 = 3.0 / 1_000_000.0;

/// Estimated USD cost per token (cached input).
const COST_PER_TOKEN_CACHED: f64 = 0.30 / 1_000_000.0;

/// Estimate tokens from file count and average bytes per file.
/// Uses the same 1-token-per-4-bytes heuristic.
pub fn estimate_from_file_count(file_count: usize, avg_bytes: usize) -> usize {
    (file_count * avg_bytes).div_ceil(4)
}

/// Heuristic token estimate: roughly 1 token per 4 characters.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Estimate USD cost for a given token count.
pub fn estimate_cost(tokens: usize, cached: bool) -> f64 {
    let rate = if cached {
        COST_PER_TOKEN_CACHED
    } else {
        COST_PER_TOKEN_UNCACHED
    };
    tokens as f64 * rate
}

/// Truncate plain-text output by lines to fit within a token budget.
/// Returns the (possibly truncated) string with a footer if truncation occurred.
pub fn truncate_to_token_budget(output: &str, budget: usize, json: bool) -> String {
    let current = estimate_tokens(output);
    if current <= budget {
        return output.to_string();
    }

    if json {
        truncate_json_to_budget(output, budget)
    } else {
        truncate_plain_to_budget(output, budget)
    }
}

fn truncate_plain_to_budget(output: &str, budget: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let total = lines.len();
    // Binary search for max lines that fit
    let mut lo: usize = 0;
    let mut hi: usize = total;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = lines[..mid].join("\n");
        let footer = format!("\n[Truncated: showed {}/{} items to stay within {} token budget]\n", mid, total, budget);
        if estimate_tokens(&format!("{}{}", candidate, footer)) <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let shown = lo;
    let mut result: String = lines[..shown].join("\n");
    result.push_str(&format!("\n[Truncated: showed {}/{} items to stay within {} token budget]\n", shown, total, budget));
    result
}

fn truncate_json_to_budget(output: &str, budget: usize) -> String {
    // Try to parse as JSON array; if not, fall back to plain truncation
    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(output);
    match parsed {
        Ok(items) => {
            let total = items.len();
            // Binary search for max items
            let mut lo: usize = 0;
            let mut hi: usize = total;
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                let slice = &items[..mid];
                let candidate = serde_json::to_string(slice).unwrap_or_default();
                let footer = format!("\n[Truncated: showed {}/{} items to stay within {} token budget]\n", mid, total, budget);
                if estimate_tokens(&format!("{}{}", candidate, footer)) <= budget {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            let shown = lo;
            let mut result = serde_json::to_string(&items[..shown]).unwrap_or_else(|_| "[]".to_string());
            result.push_str(&format!("\n[Truncated: showed {}/{} items to stay within {} token budget]\n", shown, total, budget));
            result
        }
        Err(_) => {
            // Not an array, fall back to plain text truncation
            truncate_plain_to_budget(output, budget)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn short_string() {
        // 5 chars → (5+3)/4 = 2 tokens
        assert_eq!(estimate_tokens("hello"), 2);
    }

    #[test]
    fn exact_multiple() {
        // 8 chars → (8+3)/4 = 2 tokens
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn cost_uncached() {
        let cost = estimate_cost(1_000_000, false);
        assert!((cost - 3.0).abs() < 1e-9);
    }

    #[test]
    fn cost_cached() {
        let cost = estimate_cost(1_000_000, true);
        assert!((cost - 0.30).abs() < 1e-9);
    }

    #[test]
    fn estimate_from_file_count_basic() {
        // 100 files * 2000 bytes = 200_000 bytes / 4 = 50_000 tokens
        assert_eq!(estimate_from_file_count(100, 2000), 50_000);
    }

    #[test]
    fn estimate_from_file_count_zero() {
        assert_eq!(estimate_from_file_count(0, 2000), 0);
        assert_eq!(estimate_from_file_count(100, 0), 0);
    }

    #[test]
    fn estimate_from_file_count_rounds_up() {
        // 1 file * 5 bytes = 5 / 4 = 2 (rounded up)
        assert_eq!(estimate_from_file_count(1, 5), 2);
    }

    #[test]
    fn cost_zero_tokens() {
        assert!((estimate_cost(0, false)).abs() < 1e-12);
        assert!((estimate_cost(0, true)).abs() < 1e-12);
    }

    #[test]
    fn truncate_no_op_when_under_budget() {
        let text = "short text";
        let result = truncate_to_token_budget(text, 1000, false);
        assert_eq!(result, text);
    }

    #[test]
    fn truncate_plain_text_over_budget() {
        // Each line ~10 chars = ~3 tokens. 10 lines = ~30 tokens.
        let lines: Vec<String> = (0..10).map(|i| format!("line {:04}", i)).collect();
        let text = lines.join("\n");
        let full_tokens = estimate_tokens(&text);
        // Use a budget smaller than full output but large enough for footer + some lines
        let budget = full_tokens / 2 + 10; // leave room for footer
        let result = truncate_to_token_budget(&text, budget, false);
        assert!(result.contains("[Truncated:"));
        assert!(result.contains("token budget]"));
        assert!(estimate_tokens(&result) <= budget);
        // Should have fewer than 10 lines
        let content_lines = result.lines().filter(|l| !l.starts_with("[Truncated:")).count();
        assert!(content_lines < 10);
    }

    #[test]
    fn truncate_json_array_over_budget() {
        let items: Vec<serde_json::Value> = (0..20)
            .map(|i| serde_json::json!({"name": format!("item_{}", i), "value": i}))
            .collect();
        let json = serde_json::to_string(&items).unwrap();
        let result = truncate_to_token_budget(&json, 50, true);
        assert!(result.contains("[Truncated:"));
        assert!(result.contains("20 items"));
    }

    #[test]
    fn truncate_plain_no_budget() {
        let text = "hello\nworld\n";
        assert_eq!(truncate_to_token_budget(text, 10000, false), text);
    }
}
