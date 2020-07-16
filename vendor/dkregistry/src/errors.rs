//! Error chains, types and traits.

error_chain! {
    foreign_links {
        Base64Decode(base64::DecodeError);
        HeaderInvalid(http::header::InvalidHeaderValue);
        HeaderParse(http::header::ToStrError);
        Hyper(http::Error);
        Io(std::io::Error);
        Json(serde_json::Error);
        Regex(regex::Error);
        Reqwest(reqwest::Error);
        UriParse(http::uri::InvalidUri);
        Utf8Parse(std::string::FromUtf8Error);
        StrumParse(strum::ParseError);
    }
}
