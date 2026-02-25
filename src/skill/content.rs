/// The shared codehud skill content, compiled into the binary.
pub const SKILL_CONTENT: &str = include_str!("skill_content.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_content_is_not_empty() {
        assert!(!SKILL_CONTENT.is_empty());
        assert!(SKILL_CONTENT.contains("codehud"));
    }
}
