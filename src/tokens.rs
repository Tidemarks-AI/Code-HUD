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
}
