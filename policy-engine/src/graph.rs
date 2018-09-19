// Copyright 2018 Alex Crawford
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use cincinnati::{CONTENT_TYPE, Graph};
use failure::Error;
use futures::{future, Future, Stream};
use hyper::{Body, Client, Request, Uri};
use serde_json;

pub fn index(req: HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    match req.headers().get(header::ACCEPT) {
        Some(entry) if entry == HeaderValue::from_static(CONTENT_TYPE) => Box::new(
            Client::new()
                .request(
                    Request::get(&req.state().upstream)
                        .header(header::ACCEPT, HeaderValue::from_static(CONTENT_TYPE))
                        .body(Body::empty())
                        .expect("unable to form request"),
                )
                .from_err::<Error>()
                .and_then(|res| {
                    if res.status().is_success() {
                        future::ok(res)
                    } else {
                        future::err(format_err!(
                            "failed to fetch upstream graph: {}",
                            res.status()
                        ))
                    }
                })
                .and_then(|res| res.into_body().concat2().from_err::<Error>())
                .and_then(|body| {
                    let graph: Graph = serde_json::from_slice(&body)?;
                    Ok(HttpResponse::Ok()
                        .content_type(CONTENT_TYPE)
                        .body(serde_json::to_string(&graph)?))
                }),
        ),
        _ => Box::new(future::ok(HttpResponse::NotAcceptable().finish())),
    }
}

#[derive(Clone)]
pub struct State {
    pub upstream: Uri,
}
