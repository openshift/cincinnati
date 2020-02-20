//! Test helpers.

use failure::Fallible;
use tokio::runtime::Runtime;

/// Initialize logging.
pub fn init_logger() -> Fallible<()> {
    env_logger::try_init_from_env(env_logger::Env::default())?;
    Ok(())
}

/// Initialize a tokio runtime for tests, with logging.
pub fn init_runtime() -> Fallible<Runtime> {
    let _ = init_logger();
    Runtime::new().map_err(failure::Error::from)
}

/// Register a dummy gauge, with given value.
pub fn dummy_gauge(registry: &prometheus::Registry, value: f64) -> Fallible<()> {
    let test_gauge = prometheus::Gauge::new("dummy_gauge", "dummy help")?;
    test_gauge.set(value);
    registry.register(Box::new(test_gauge))?;
    Ok(())
}

/// Sort the JSON value represantion of a graph by version.
pub fn sort_json_graph_by_version(v: &mut serde_json::Value) {
    if !v.is_object() {
        panic!("not an object");
    }
    let obj = v.as_object_mut().unwrap();

    let mut index_map = std::collections::HashMap::<usize, usize>::new();

    {
        let mut version_index =
            std::collections::HashMap::<String, (Option<usize>, Option<usize>)>::new();

        let nodes = obj.get_mut("nodes").unwrap();
        nodes
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .for_each(|(i, node)| {
                let version = node.get("version").unwrap().as_str().unwrap();
                version_index.insert(version.to_string(), (Some(i), None));
            });

        nodes.as_array_mut().unwrap().sort_unstable_by(|a, b| {
            a.get("version")
                .unwrap()
                .as_str()
                .cmp(&b.get("version").unwrap().as_str())
        });

        nodes
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .for_each(|(i, node)| {
                let version = node.get("version").unwrap().as_str().unwrap();
                version_index
                    .entry(version.to_string())
                    .and_modify(|(from, to)| {
                        *to = Some(i);
                        println!(
                            "{} changed index from {} to {}",
                            version,
                            from.unwrap(),
                            to.unwrap()
                        );
                        index_map.insert(from.unwrap(), to.unwrap());
                    });
            });
    }

    obj.get_mut("edges")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .for_each(|ref mut edge: &mut serde_json::Value| {
            if edge.as_array().unwrap().len() < 2 {
                return;
            }

            macro_rules! rewrite_edge_index {
                ($edge_index:expr) => {
                    let old = edge.get_mut($edge_index).unwrap();
                    let new = {
                        let old_usize = old.as_u64().unwrap() as usize;
                        let new_i64 = *index_map.get(&old_usize).unwrap() as i64;
                        serde_json::Value::Number(serde_json::Number::from(new_i64))
                    };
                    println!("Rewriting {:?} -> {:?})", old, new);
                    *old = new;
                };
            }

            rewrite_edge_index!(0);
            rewrite_edge_index!(1);
        });

    // Sort the edges array. This is necessary in case of nodes having multiple edges.
    obj.get_mut("edges")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .sort_unstable_by(|a, b| {
            let a0 = a.get(0).unwrap().as_u64().unwrap();
            let a1 = a.get(1).unwrap().as_u64().unwrap();
            let b0 = b.get(0).unwrap().as_u64().unwrap();
            let b1 = b.get(1).unwrap().as_u64().unwrap();

            a0.cmp(&b0).then_with(|| a1.cmp(&b1))
        });
}
