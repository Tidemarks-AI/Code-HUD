/// This is a documented struct.
/// It has multiple lines.
pub struct DocStruct {
    pub field: u32,
}

/** Block doc comment for a function. */
pub fn block_documented() -> bool {
    true
}

// Regular comment, not a doc comment.
pub fn not_documented() {}

/// Doc on impl method.
impl DocStruct {
    /// Method doc comment.
    pub fn method(&self) -> u32 {
        self.field
    }
}
