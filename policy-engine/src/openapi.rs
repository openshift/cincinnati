use crate::AppState;
use actix_web::{HttpRequest, HttpResponse};
use openapiv3::{OpenAPI, ReferenceOr};
use std::collections::HashSet;

/// Template for policy-engine OpenAPIv3 document.
const SPEC: &str = include_str!("openapiv3.json");

pub(crate) fn index(req: HttpRequest) -> HttpResponse {
    let path_prefix = &req
        .app_data::<AppState>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .path_prefix;

    let mut spec_object: OpenAPI = match serde_json::from_str(SPEC) {
        Ok(o) => o,
        Err(e) => {
            let e = format_err!("Could not deserialize to OpenAPI object: {}", e);
            error!("{}", e);
            return HttpResponse::from_error(e.into());
        }
    };

    // Add mandatory parameters to the `graph` endpoint.
    if let Some(path) = spec_object.paths.get_mut("/v1/graph") {
        add_mandatory_params(
            path,
            &req.app_data::<AppState>()
                .expect(commons::MISSING_APPSTATE_PANIC_MSG)
                .mandatory_params,
        );
    }

    // Prefix all paths with `path_prefix`
    spec_object.paths = rewrite_paths(spec_object.paths, path_prefix);

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
    use super::*;

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

    #[test]
    fn graph_params_integration() -> Result<(), Box<std::error::Error>> {
        // prepare and run the test-service
        let service_uri = "/openapi";
        let mandatory_params: HashSet<String> = ["MARKER1", "MARKER2"]
            .iter()
            .cloned()
            .map(String::from)
            .collect();
        let path_prefix = "test_prefix".to_string();

        let data = actix_web::web::Data::new(AppState {
            mandatory_params: mandatory_params.clone(),
            path_prefix: path_prefix.clone(),
            plugins: Box::leak(Box::new([])),
            upstream: Default::default(),
        });
        let resource =
            actix_web::web::resource(service_uri).route(actix_web::web::get().to(super::index));
        let app = actix_web::App::new().register_data(data).service(resource);

        let mut svc = actix_web::test::init_service(app);

        // call the service and get the response body
        let body = {
            let mut response = actix_web::test::call_service(
                &mut svc,
                actix_web::test::TestRequest::with_uri(&service_uri)
                    .header("Accept", "application/json")
                    .to_request(),
            );

            if response.status() != actix_web::http::StatusCode::OK {
                return Err(format!("unexpected statuscode:{}", response.status()).into());
            };

            let body = match response.take_body() {
                actix_web::dev::ResponseBody::Body(b) => match b {
                    actix_web::dev::Body::Bytes(bytes) => bytes,
                    unknown => {
                        return Err(format!("expected byte body, got '{:?}'", unknown).into())
                    }
                },
                _ => return Err("expected body response".into()),
            };

            std::str::from_utf8(&body)?.to_owned()
        };

        // parse the response and extract the required parameters
        let spec: openapiv3::OpenAPI = serde_json::from_str(&body)?;
        let v1_graph: &openapiv3::ReferenceOr<openapiv3::PathItem> = spec
            .paths
            .get(&format!("{}/v1/graph", path_prefix))
            .ok_or("could not find /v1/graph endpoint in openapi spec")?;

        let v1_graph_mandatory_params_result: HashSet<String> = match v1_graph {
            ReferenceOr::Item(item) => item
                .parameters
                .iter()
                .filter_map(|param| {
                    if let ReferenceOr::Item(openapiv3::Parameter::Query {
                        parameter_data, ..
                    }) = param
                    {
                        if parameter_data.required {
                            return Some(parameter_data.name.clone());
                        }
                    };

                    None
                })
                .collect(),
            _ => return Err("reference manipulation for paths not allowed".into()),
        };

        assert_eq!(mandatory_params, v1_graph_mandatory_params_result,);

        Ok(())
    }
}
