use actix_web::{HttpRequest, HttpResponse};
use openapiv3::OpenAPI;
use AppState;

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

#[cfg(test)]
mod tests {

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
}
