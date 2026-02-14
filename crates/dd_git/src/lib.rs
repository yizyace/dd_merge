pub fn name() -> &'static str {
    "dd_git"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(name(), "dd_git");
    }
}
