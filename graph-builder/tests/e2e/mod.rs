#[cfg(feature = "test-e2e")]
use assert_json_diff::assert_json_include;
use reqwest::header::{HeaderValue, ACCEPT};
use serde_json::Value;
use std::env;
use test_case::test_case;
use url::Url;

pub fn sort_by_version(v: &mut Value) {
    if !v.is_object() {
        return;
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
        })
}

#[test_case("a",    "amd64", include_str!("./testdata/a-amd64.json");    "channel a amd64")]
#[test_case("b",    "amd64", include_str!("./testdata/b-amd64.json");    "channel b amd64")]
#[test_case("test", "amd64", include_str!("./testdata/test-amd64.json"); "channel test amd64")]
fn e2e_channel_success(channel: &'static str, arch: &'static str, testdata: &str) {
    let mut runtime = commons::testing::init_runtime().unwrap();

    let graph_base_url = match env::var("GRAPH_URL") {
        Ok(env) => env,
        _ => panic!("GRAPH_URL unset"),
    };

    let mut expected: Value = serde_json::from_str(testdata).unwrap();
    sort_by_version(&mut expected);

    let mut graph_url = Url::parse(&graph_base_url).unwrap();
    graph_url
        .query_pairs_mut()
        .append_pair("channel", channel)
        .append_pair("arch", arch);

    println!("Querying {}", graph_url.to_string());

    let res = runtime
        .block_on(
            reqwest::ClientBuilder::new()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap()
                .get(&graph_url.to_string())
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .send(),
        )
        .unwrap();
    assert_eq!(res.status().is_success(), true);
    let text = runtime.block_on(res.text()).unwrap();
    let mut actual = serde_json::from_str(&text).unwrap();
    sort_by_version(&mut actual);
    assert_json_include!(actual: actual, expected: expected)
}
