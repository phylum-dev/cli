extern crate hyper;
extern crate phylum_cli;

#[macro_use]
extern crate serde_derive;

use phylum_cli::restson::{Error, RestClient, RestPath};

#[derive(Deserialize)]
struct HttpBinAnything {
    headers: TestHeaders,
}

#[derive(Deserialize)]
struct TestHeaders {
    #[serde(default)]
    #[serde(rename = "User-Agent")]
    user_agent: String,

    #[serde(default)]
    #[serde(rename = "X-Test")]
    test: String,
}

impl RestPath<()> for HttpBinAnything {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(String::from("anything"))
    }
}

#[test]
fn headers() {
    let mut client = RestClient::new("http://httpbin.org").unwrap();

    client.set_header("user-agent", "restson-test").unwrap();

    let data: HttpBinAnything = client.get(()).unwrap();
    assert_eq!(data.headers.user_agent, "restson-test");
}

#[test]
fn headers_clear() {
    let mut client = RestClient::new("http://httpbin.org").unwrap();

    client.set_header("X-Test", "12345").unwrap();

    let data: HttpBinAnything = client.get(()).unwrap();
    assert_eq!(data.headers.test, "12345");

    client.clear_headers();

    let data: HttpBinAnything = client.get(()).unwrap();
    assert_eq!(data.headers.test, "");
}

#[test]
fn default_user_agent() {
    let mut client = RestClient::new("http://httpbin.org").unwrap();

    let data: HttpBinAnything = client.get(()).unwrap();
    assert_eq!(
        data.headers.user_agent,
        "phylum-cli/".to_owned() + env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn response_headers() {
    let mut client = RestClient::new("http://httpbin.org").unwrap();

    let _data: HttpBinAnything = client.get(()).unwrap();
    assert_eq!(
        client.response_headers()["content-type"],
        "application/json"
    );
}
