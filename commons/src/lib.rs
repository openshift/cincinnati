/// Strip any leading and trailing slashes
pub fn parse_path_namespace(path_namespace: &str) -> String {
    path_namespace.to_string().trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_path_namespace() {
        assert_eq!(super::parse_path_namespace("//a/b/c/"), "a/b/c");
    }
}
