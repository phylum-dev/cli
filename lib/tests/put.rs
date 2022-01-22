extern crate phylum_cli;

#[macro_use]
extern crate serde_derive;

use phylum_cli::restson::{Error, RestClient, RestPath};

mod logging;

#[derive(Serialize, Deserialize)]
struct HttpBinPut {
    data: String,
}

#[derive(Deserialize)]
struct HttpBinPutResp {
    json: HttpBinPut,
    url: String,
}

impl RestPath<()> for HttpBinPut {
    fn get_path(_: ()) -> Result<String, Error> {
        Ok(String::from("put"))
    }
}

#[tokio::test]
async fn basic_put() {
    let mut client = RestClient::new("https://httpbin.org").unwrap();

    let data = HttpBinPut {
        data: String::from("test data"),
    };
    client.put((), &data).await.unwrap();
}

#[tokio::test]
async fn put_query_params() {
    let mut client = RestClient::new("https://httpbin.org").unwrap();

    let params = vec![("a", "2"), ("b", "abcd")];
    let data = HttpBinPut {
        data: String::from("test data"),
    };
    client.put_with((), &data, &params).await.unwrap();
}

#[tokio::test]
async fn put_capture() {
    let mut client = RestClient::new("https://httpbin.org").unwrap();

    let data = HttpBinPut {
        data: String::from("test data"),
    };
    let resp: HttpBinPutResp = client.put_capture((), &data).await.unwrap();

    assert_eq!(resp.json.data, "test data");
    assert_eq!(resp.url, "https://httpbin.org/put");
}

#[tokio::test]
async fn put_capture_query_params() {
    let mut client = RestClient::new("https://httpbin.org").unwrap();

    let params = vec![("a", "2"), ("b", "abcd")];
    let data = HttpBinPut {
        data: String::from("test data"),
    };
    let resp: HttpBinPutResp = client.put_capture_with((), &data, &params).await.unwrap();

    assert_eq!(resp.json.data, "test data");
    assert_eq!(resp.url, "https://httpbin.org/put?a=2&b=abcd");
}
