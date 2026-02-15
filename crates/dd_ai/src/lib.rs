pub fn name() -> &'static str {
    "dd_ai"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "dd_ai");
    }
}
