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
                        return Err(Box::new(format!("{}", $key)));
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
                                return Err(Box::new(format!("{}", $key)));
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
macro_rules! try_get_multiple_values {
    ($map:expr, $key:expr ) => {
        {
            let closure = || {
                (
                    $map.get($key)
                )
            };
            closure()
        }
    };
    ($map:expr, $( $key:expr ),* ) => {
        {
            let closure = || {
                (
                    (
                        $(
                            $map.get($key),
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

#[macro_export]
macro_rules! plugin_config {
    ($( $tuple:expr ),*) => {
        cincinnati::plugins::catalog::deserialize_config(toml::value::Value::Table(toml::value::Table::from_iter(
            [ $(($tuple)),* ]
            .iter()
            .map(|(k, v)| (k.to_string(), toml::value::Value::String(v.to_string()))),
        )))
    };
}

#[macro_export]
macro_rules! plugin_config_option {
    ($( $tuple:expr ),*) => {
        cincinnati::plugins::catalog::deserialize_config(toml::value::Value::Table(toml::value::Table::from_iter(
            [ $(($tuple)),* ]
            .iter()
            .filter_map(|kv| kv.as_ref())
            .map(|(k, v)| (k.to_string(), toml::value::Value::String(v.to_string()))),
        )))
    };
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

    #[test]
    fn ensure_try_get_multiple_values() {
        let params = [
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<String, String>>();

        let (a, b, c) = try_get_multiple_values!(params, "a", "b", "c");
        assert_eq!(
            (Some(&"a".to_string()), Some(&"b".to_string()), None),
            (a, b, c)
        );
    }

    #[test]
    fn get_try_multiple_values_single() {
        let params = [
            ("a".to_string(), "a".to_string()),
            ("b".to_string(), "b".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<String, String>>();

        let a = try_get_multiple_values!(params, "a");
        assert_eq!(Some(&"a".to_string()), a);

        let c = try_get_multiple_values!(params, "c");
        assert!(c.is_none());
    }
}
