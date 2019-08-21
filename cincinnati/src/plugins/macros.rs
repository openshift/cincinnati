//! Macro definitions to be used by all plugins

#[macro_export]
macro_rules! get_multiple_values {
    ($map:expr, $key:expr ) => {
        {
            let closure = || {
                Ok(
                    if let Some(value) = $map.get($key) {
                        value
                    } else {
                        bail!("could not find key '{}'", $key)
                    },
                )
            };
            closure()
        }
    };
    ($map:expr, $( $key:expr ),* ) => {
        {
            let closure = || {
                Ok(
                    (
                        $(
                            if let Some(value) = $map.get($key) {
                                value
                            } else {
                                bail!("could not find key '{}'", $key)
                            },
                        )*
                    )
                )
            };
            closure()
        }
    }
}

#[macro_export]
macro_rules! new_plugin {
    ($x:expr) => {
        Box::new($x)
    };
}

#[macro_export]
macro_rules! new_plugins {
    ($($x:expr),*) => { vec![$(new_plugin!($x)),*] };
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn ensure_get_multiple_values() {
        let params = [
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<String, String>>();

        let (a, b): (&String, &String) = get_multiple_values!(params, "a", "b").unwrap();
        assert_eq!((&"a".to_string(), &"b".to_string()), (a, b));

        let params = [
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<String, String>>();

        assert!(get_multiple_values!(params, "c").is_err());
    }

    #[test]
    fn get_multiple_values_single() {
        let params = [
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<String, String>>();

        let a: &String = get_multiple_values!(params, "a").unwrap();
        assert_eq!(&"a".to_string(), a);
    }

}
