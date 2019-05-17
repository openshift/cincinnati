use actix_web::{HttpRequest, HttpResponse};
use openapiv3::{OpenAPI, ReferenceOr};
use std::collections::HashSet;
use crate::AppState;

/// Template for policy-engine OpenAPIv3 document.
const SPEC: &str = include_str!("openapiv3.json");

pub(crate) fn index(req: HttpRequest<AppState>) -> HttpResponse {
    let path_prefix = &req.state().path_prefix;

    let mut spec_object: OpenAPI = match serde_json::from_str(SPEC) {
        Ok(o) => o,
        Err(e) => {
            let e = format_err!("Could not deserialize to OpenAPI object: {}", e);
            error!("{}", e);
            return HttpResponse::from_error(e.into());
        }
    };

    // Prefix all paths with `path_prefix`
    spec_object.paths = rewrite_paths(spec_object.paths, path_prefix);

    // Add mandatory parameters to the `graph` endpoint.
    if let Some(path) = spec_object.paths.get_mut("/v1/graph") {
        add_mandatory_params(path, &req.state().mandatory_params);
    }

    match serde_json::to_string(&spec_object) {
        Ok(s) => HttpResponse::from(s),
        Err(e) => {
            let e = format_err!("Could not serialize OpenAPI object: {}", e);
            error!("{}", e);
            HttpResponse::from_error(e.into())
        }
    }
}

fn rewrite_paths(paths: openapiv3::Paths, path_prefix: &str) -> openapiv3::Paths {
    paths
        .into_iter()
        .map(|(path, path_item)| {
            let new_path = format!("{}{}", path_prefix, &path);
            trace!("Rewrote path {} -> {} ", &path, &new_path);
            (new_path, path_item)
        })
        .collect()
}

// Add mandatory parameters to the `graph` endpoint.
fn add_mandatory_params(path: &mut ReferenceOr<openapiv3::PathItem>, reqs: &HashSet<String>) {
    // Template for building an `openapiv3::Parameter`, which otherwise has private fields.
    static PARAM_TEMPLATE: &str = r#"
{
    "in": "query",
    "name": "TEMPLATE",
    "required": true,
    "schema": {
        "type": "string"
    }
}
"#;

    match path {
        ReferenceOr::Item(item) => {
            let template: openapiv3::Parameter =
                serde_json::from_str(PARAM_TEMPLATE).expect("hardcoded deserialization failed");
            for key in reqs {
                let mut data = template.clone();
                match data {
                    openapiv3::Parameter::Query {
                        parameter_data: ref mut p,
                        ..
                    } => {
                        p.name = key.clone();
                    }
                    _ => {
                        error!("non-query parameters not allowed");
                        continue;
                    }
                };
                let param = ReferenceOr::Item(data);
                item.parameters.push(param);
            }
        }
        _ => error!("reference manipulation for paths not allowed"),
    };
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn test_rewrite_paths() {
        use super::{rewrite_paths, SPEC};
        use openapiv3::OpenAPI;

        let prefix = "/test_prefix";
        let spec_object: OpenAPI = serde_json::from_str(SPEC).expect("couldn't parse JSON file");

        let paths_before = spec_object.paths;
        let paths_after = rewrite_paths(paths_before.clone(), &prefix);

        paths_before.iter().zip(paths_after.iter()).for_each(
            |((path_before, _), (path_after, _))| {
                assert_ne!(path_after, path_before);
                assert_eq!(path_after, &format!("{}{}", prefix, path_before));
                assert!(path_after.as_str().starts_with(prefix));
            },
        );
    }

    #[test]
    fn graph_params() {
        use super::{add_mandatory_params, SPEC};
        use openapiv3::OpenAPI;

        let params: HashSet<String> = vec!["MARKER1".to_string(), "MARKER2".to_string()]
            .into_iter()
            .collect();
        let mut spec: OpenAPI = serde_json::from_str(SPEC).expect("couldn't parse JSON file");

        {
            let mut graph_path = spec.paths.get_mut("/v1/graph").unwrap();
            add_mandatory_params(&mut graph_path, &params);
        }
        let output = serde_json::to_string(&spec).unwrap();

        for p in params {
            assert!(
                output.contains(&p),
                "marker {} not found in output: {}",
                p,
                output
            )
        }
    }
}
