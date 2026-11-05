use crate::CompilerError;

/// Default cap on shader keyword combinations.
///
/// Each declared keyword doubles the number of specialized programs. A cook
/// that would exceed this cap fails instead of compiling them.
pub const DEFAULT_VARIANT_BUDGET: u32 = 64;

/// Returns an error when `2^keywords` is greater than `budget`.
pub fn check_variant_budget(keywords: &[&str], budget: u32) -> Result<(), CompilerError> {
    let combinations = if keywords.len() >= 31 {
        u32::MAX
    } else {
        1u32 << keywords.len()
    };
    if combinations > budget {
        Err(CompilerError::VariantBudget(combinations, budget))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_keywords_fit_and_eight_do_not() {
        let seven = ["a", "b", "c", "d", "e", "f", "g"];
        assert!(check_variant_budget(&seven[..6], DEFAULT_VARIANT_BUDGET).is_ok());
        assert!(check_variant_budget(&seven, DEFAULT_VARIANT_BUDGET).is_err());
    }
}
