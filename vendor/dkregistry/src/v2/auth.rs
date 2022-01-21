use crate::errors::{Error, Result};
use crate::v2::*;
use reqwest::{header::HeaderValue, RequestBuilder, StatusCode, Url};

/// Represents all supported authentication schemes and is stored by `Client`.
#[derive(Debug, Clone)]
pub enum Auth {
    Bearer(BearerAuth),
    Basic(BasicAuth),
}

impl Auth {
    /// Add authentication headers to a request builder.
    pub(crate) fn add_auth_headers(&self, request_builder: RequestBuilder) -> RequestBuilder {
        match self {
            Auth::Bearer(bearer_auth) => request_builder.bearer_auth(bearer_auth.token.clone()),
            Auth::Basic(basic_auth) => {
                request_builder.basic_auth(basic_auth.user.clone(), basic_auth.password.clone())
            }
        }
    }
}

/// Used for Bearer HTTP Authentication.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BearerAuth {
    token: String,
    expires_in: Option<u32>,
    issued_at: Option<String>,
    refresh_token: Option<String>,
}

impl BearerAuth {
    async fn try_from_header_content(
        client: Client,
        scopes: &[&str],
        credentials: Option<(String, String)>,
        bearer_header_content: WwwAuthenticateHeaderContentBearer,
    ) -> Result<Self> {
        let auth_ep = bearer_header_content.auth_ep(scopes);
        trace!("authenticate: token endpoint: {}", auth_ep);

        let url = reqwest::Url::parse(&auth_ep)?;

        let auth_req = {
            Client {
                auth: credentials.map(|(user, password)| {
                    Auth::Basic(BasicAuth {
                        user,
                        password: Some(password),
                    })
                }),
                ..client
            }
        }
        .build_reqwest(Method::GET, url);

        let r = auth_req.send().await?;
        let status = r.status();
        trace!("authenticate: got status {}", status);
        if status != StatusCode::OK {
            return Err(Error::UnexpectedHttpStatus(status));
        }

        let bearer_auth = r.json::<BearerAuth>().await?;

        match bearer_auth.token.as_str() {
            "unauthenticated" | "" => return Err(Error::InvalidAuthToken(bearer_auth.token)),
            _ => {}
        };

        // mask the token before logging it
        let chars_count = bearer_auth.token.chars().count();
        let mask_start = std::cmp::min(1, chars_count - 1);
        let mask_end = std::cmp::max(chars_count - 1, 1);
        let mut masked_token = bearer_auth.token.clone();
        masked_token.replace_range(mask_start..mask_end, &"*".repeat(mask_end - mask_start));

        trace!("authenticate: got token: {:?}", masked_token);

        Ok(bearer_auth)
    }
}

/// Used for Basic HTTP Authentication.
#[derive(Debug, Clone)]
pub struct BasicAuth {
    user: String,
    password: Option<String>,
}

/// Structured representation for the content of the authentication response header.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all(deserialize = "lowercase"))]
pub(crate) enum WwwAuthenticateHeaderContent {
    Bearer(WwwAuthenticateHeaderContentBearer),
    Basic(WwwAuthenticateHeaderContentBasic),
}

const REGEX: &str = r#"(?x)\s*
((?P<method>[A-Za-z]+)\s)?
\s*
(
        (?P<key>[A-Za-z]+)
    \s*
        =
    \s*
        "(?P<value>[^"]+)"
    \s*
)
"#;

#[derive(Debug, thiserror::Error)]
pub enum WwwHeaderParseError {
    #[error("header value must conform to {}", REGEX)]
    InvalidValue,
    #[error("'method' field missing")]
    FieldMethodMissing,
}

impl WwwAuthenticateHeaderContent {
    /// Create a `WwwAuthenticateHeaderContent` by parsing a `HeaderValue` instance.
    pub(crate) fn from_www_authentication_header(header_value: HeaderValue) -> Result<Self> {
        let header = String::from_utf8(header_value.as_bytes().to_vec())?;

        // This regex will result in multiple captures which will contain one key-value pair each.
        // The first capture will be the only one with the "method" group set.
        let re = regex::Regex::new(REGEX).expect("this static regex is valid");
        let captures = re.captures_iter(&header).collect::<Vec<_>>();

        let method = captures
            .get(0)
            .ok_or(WwwHeaderParseError::InvalidValue)?
            .name("method")
            .ok_or(WwwHeaderParseError::FieldMethodMissing)?
            .as_str()
            .to_lowercase();

        let serialized_content = {
            let serialized_captures = captures
                .iter()
                .filter_map(|capture| {
                    match (
                        capture.name("key").map(|n| n.as_str().to_lowercase()),
                        capture.name("value").map(|n| n.as_str().to_string()),
                    ) {
                        (Some(key), Some(value)) => Some(format!(
                            r#"{}: {}"#,
                            serde_json::Value::String(key),
                            serde_json::Value::String(value),
                        )),
                        _ => None,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                r#"{{ {}: {{ {} }} }}"#,
                serde_json::Value::String(method),
                serialized_captures
            )
        };

        // Deserialize the content
        let mut unsupported_keys = std::collections::HashSet::new();
        let content: WwwAuthenticateHeaderContent = serde_ignored::deserialize(
            &mut serde_json::Deserializer::from_str(&serialized_content),
            |path| {
                unsupported_keys.insert(path.to_string());
            },
        )?;

        if !unsupported_keys.is_empty() {
            warn!(
                "skipping unrecognized keys in authentication header: {:#?}",
                unsupported_keys
            );
        }

        Ok(content)
    }
}

/// Structured content for the Bearer authentication response header.
#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct WwwAuthenticateHeaderContentBearer {
    realm: String,
    service: Option<String>,
    scope: Option<String>,
}

impl WwwAuthenticateHeaderContentBearer {
    fn auth_ep(&self, scopes: &[&str]) -> String {
        let service = self
            .service
            .as_ref()
            .map(|sv| format!("?service={}", sv))
            .unwrap_or_default();

        let scope = scopes
            .iter()
            .enumerate()
            .fold(String::new(), |acc, (i, &s)| {
                let separator = if i > 0 { "&" } else { "" };
                acc + separator + "scope=" + s
            });

        let scope_prefix = if scopes.is_empty() {
            ""
        } else if service.is_empty() {
            "?"
        } else {
            "&"
        };

        format!("{}{}{}{}", self.realm, service, scope_prefix, scope)
    }
}

/// Structured content for the Basic authentication response header.
#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct WwwAuthenticateHeaderContentBasic {
    realm: String,
}

impl Client {
    /// Make a request and return the response's www authentication header.
    async fn get_www_authentication_header(&self) -> Result<HeaderValue> {
        let url = {
            let ep = format!("{}/v2/", self.base_url.clone(),);
            reqwest::Url::parse(&ep)?
        };

        let r = self.build_reqwest(Method::GET, url.clone()).send().await?;

        trace!("GET '{}' status: {:?}", r.url(), r.status());
        r.headers()
            .get(reqwest::header::WWW_AUTHENTICATE)
            .ok_or(Error::MissingAuthHeader("WWW-Authenticate"))
            .map(ToOwned::to_owned)
    }

    /// Perform registry authentication and return the authenticated client.
    ///
    /// If Bearer authentication is used the returned client will be authorized for the requested scopes.
    pub async fn authenticate(mut self, scopes: &[&str]) -> Result<Self> {
        let credentials = self.credentials.clone();

        let client = Client {
            auth: None,
            ..self.clone()
        };

        let authentication_header = client.get_www_authentication_header().await?;
        let auth = match WwwAuthenticateHeaderContent::from_www_authentication_header(
            authentication_header,
        )? {
            WwwAuthenticateHeaderContent::Basic(_) => {
                let basic_auth = credentials
                    .map(|(user, password)| BasicAuth {
                        user,
                        password: Some(password),
                    })
                    .ok_or(Error::NoCredentials)?;

                Auth::Basic(basic_auth)
            }
            WwwAuthenticateHeaderContent::Bearer(bearer_header_content) => {
                let bearer_auth = BearerAuth::try_from_header_content(
                    client,
                    scopes,
                    credentials,
                    bearer_header_content,
                )
                .await?;

                Auth::Bearer(bearer_auth)
            }
        };

        trace!("authenticate: login succeeded");
        self.auth = Some(auth);

        Ok(self)
    }

    /// Check whether the client can successfully make requests to the registry.
    ///
    /// This could be due to granted anonymous access or valid credentials.
    pub async fn is_auth(&self) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/", self.base_url.clone(),);
            Url::parse(&ep)?
        };

        let req = self.build_reqwest(Method::GET, url.clone());

        trace!("Sending request to '{}'", url);
        let resp = req.send().await?;
        trace!("GET '{:?}'", resp);

        let status = resp.status();
        match status {
            reqwest::StatusCode::OK => Ok(true),
            reqwest::StatusCode::UNAUTHORIZED => Ok(false),
            _ => Err(Error::UnexpectedHttpStatus(status)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn bearer_realm_parses_correctly() -> Result<()> {
        let realm = "https://sat-r220-02.lab.eng.rdu2.redhat.com/v2/token";
        let service = "sat-r220-02.lab.eng.rdu2.redhat.com";
        let scope = "repository:registry:pull,push";

        for header_value in [
            HeaderValue::from_str(&format!(
                r#"Bearer realm="{}",service="{}",scope="{}""#,
                realm, service, scope
            ))
            .unwrap(),
            HeaderValue::from_str(&format!(
                r#"bearer realm="{}",service="{}",scope="{}""#,
                realm, service, scope
            ))
            .unwrap(),
            HeaderValue::from_str(&format!(
                r#"BEARER realm="{}",service="{}",scope="{}""#,
                realm, service, scope
            ))
            .unwrap(),
            HeaderValue::from_str(&format!(
                r#"Bearer Realm="{}",Service="{}",Scope="{}""#,
                realm, service, scope
            ))
            .unwrap(),
            HeaderValue::from_str(&format!(
                r#"Bearer REALM="{}",SERVICE="{}",SCOPE="{}""#,
                realm, service, scope
            ))
            .unwrap(),
        ]
        .iter()
        {
            let content = WwwAuthenticateHeaderContent::from_www_authentication_header(
                header_value.to_owned(),
            )?;

            assert_eq!(
                WwwAuthenticateHeaderContent::Bearer(WwwAuthenticateHeaderContentBearer {
                    realm: realm.to_string(),
                    service: Some(service.to_string()),
                    scope: Some(scope.to_string()),
                }),
                content
            );
        }

        Ok(())
    }

    // Testing for this situation to work:
    // [TRACE dkregistry::v2::auth] Sending request to 'https://localhost:5000/v2/'
    // [TRACE dkregistry::v2::auth] GET 'Response { url: "https://localhost:5000/v2/", status: 401, headers: {"content-type": "application/json; charset=utf-8", "docker-distribution-api-version": "registry/2.0", "www-authenticate": "Basic realm=\"Registry\"", "x-content-type-options": "nosniff", "date": "Thu, 18 Jun 2020 09:04:24 GMT", "content-length": "87"} }'
    // [TRACE dkregistry::v2::auth] GET 'https://localhost:5000/v2/' status: 401
    // [TRACE dkregistry::v2::auth] Token provider: Registry
    // [TRACE dkregistry::v2::auth] login: token endpoint: Registry&scope=repository:cincinnati-ci/ocp-release-dev:pull
    // [ERROR graph_builder::graph] failed to fetch all release metadata
    // [ERROR graph_builder::graph] failed to parse url from string 'Registry&scope=repository:cincinnati-ci/ocp-release-dev:pull': relative URL without a base
    #[test]
    fn basic_realm_parses_correctly() -> Result<()> {
        let realm = "Registry realm";

        for header_value in [
            HeaderValue::from_str(&format!(r#"Basic realm="{}""#, realm)).unwrap(),
            HeaderValue::from_str(&format!(r#"basic realm="{}""#, realm)).unwrap(),
            HeaderValue::from_str(&format!(r#"BASIC realm="{}""#, realm)).unwrap(),
            HeaderValue::from_str(&format!(r#"Basic Realm="{}""#, realm)).unwrap(),
            HeaderValue::from_str(&format!(r#"Basic REALM="{}""#, realm)).unwrap(),
        ]
        .iter()
        {
            let content = WwwAuthenticateHeaderContent::from_www_authentication_header(
                header_value.to_owned(),
            )?;

            assert_eq!(
                WwwAuthenticateHeaderContent::Basic(WwwAuthenticateHeaderContentBasic {
                    realm: realm.to_string(),
                }),
                content
            );
        }

        Ok(())
    }

    // The following test checks the url construction within the 'auth_ep'
    // method of WwwAuthenticateHeaderContentBearer.
    // Tests that the result is correctly parsed by Url::parse and that the
    // scopes in the query string are as expected in three situations.
    // Tests combination of scopes with service query param also.
    #[test_case(&[], true; "Test with no scopes and with service")]
    #[test_case(&["repository:test:pull"], true; "Test with single scope and service")]
    #[test_case(&["repository:test:pull", "repository:example:pull,push", "repository:another:*"], false;
                "Test with multiple scopes")]
    fn bearer_auth_ep_scope_construction(scopes: &[&str], include_service: bool) {
        let realm = "https://sat-r220-02.lab.eng.rdu2.redhat.com/v2/token";
        let service = "sat-r220-02.lab.eng.rdu2.redhat.com";

        let bearer_header_content = WwwAuthenticateHeaderContentBearer {
            realm: realm.to_string(),
            service: if include_service {
                Some(service.to_string())
            } else {
                None
            },
            scope: None,
        };

        // build list of expected headers
        let mut expected_headers: Vec<(String, String)> = scopes
            .iter()
            .map(|a| ("scope".to_owned(), a.to_string()))
            .collect();
        // first one is the service header if specified
        if include_service {
            expected_headers.insert(0, ("service".to_owned(), service.to_string()));
        }

        let result = bearer_header_content.auth_ep(scopes);
        let url = Url::parse(&result).unwrap();

        assert_eq!(
            url.query_pairs().into_owned().collect::<Vec<_>>(),
            expected_headers
        );
    }
}
