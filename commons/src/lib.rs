/// Strip all but one leading slash and all trailing slashes
pub fn parse_path_prefix(path_prefix: &str) -> String {
    format!("/{}", path_prefix.to_string().trim_matches('/'))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_path_prefix() {
        assert_eq!(super::parse_path_prefix("//a/b/c//"), "/a/b/c");
        assert_eq!(super::parse_path_prefix("/a/b/c/"), "/a/b/c");
        assert_eq!(super::parse_path_prefix("/a/b/c"), "/a/b/c");
        assert_eq!(super::parse_path_prefix("a/b/c"), "/a/b/c");
    }
}
