use super::*;
use bytes::Bytes;
use mockito::Matcher;
use serde_json::json;

#[tokio::test]
async fn authenticate_test_success() {
    let username = "me@example.com";
    let password = "password";
    let name = "louis-test";
    let expected_secret = "some-secret-here";

    let resp_body = json!({
        "authing_key": false,
        "inserted_at": "2020-05-02T17:18:23.336328Z",
        "name": "authenticate_test_1",
        "permissions": [{"domain": "api", "resource": "write"}],
        "revoked_at": null,
        "secret": expected_secret,
        "updated_at": "2020-05-02T17: 18: 23.336328Z",
        "url": "https: //hex.pm/api/keys/authenticate_test_1"
    });

    let mock = mockito::mock("POST", "/keys")
        .expect(1)
        .match_header("authorization", "Basic bWVAZXhhbXBsZS5jb206cGFzc3dvcmQ=")
        .match_header("content-type", "application/json")
        .match_header("accept", "application/json")
        .match_body(Matcher::Json(json!({
            "name": name,
            "permissions":[{ "domain": "api", "resource": "write" }]
        })))
        .with_status(201)
        .with_body(resp_body.to_string())
        .create();

    let mut client = UnauthenticatedClient::new();
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    let authed_client = client
        .authenticate(username, password, name)
        .await
        .expect("should be ok");

    assert_eq!(expected_secret, authed_client.api_token);
    assert_eq!(
        url::Url::parse(&mockito::server_url()).unwrap(),
        authed_client.api_base
    );
    mock.assert();
}

#[tokio::test]
async fn authenticate_test_rate_limted() {
    let username = "me@example.com";
    let password = "password";
    let name = "authenticate_test_2";

    let mock = mockito::mock("POST", "/keys")
        .expect(1)
        .match_header("authorization", "Basic bWVAZXhhbXBsZS5jb206cGFzc3dvcmQ=")
        .match_header("content-type", "application/json")
        .match_header("accept", "application/json")
        .match_body(Matcher::Json(json!({
            "name": name,
            "permissions":[{ "domain": "api", "resource": "write" }]
        })))
        .with_status(429)
        .create();

    let mut client = UnauthenticatedClient::new();
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.authenticate(username, password, name).await {
        Err(AuthenticateError::RateLimited) => (),
        result => panic!(
            "expected Err(AuthenticateError::RateLimited), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn authenticate_test_bad_creds() {
    let username = "me@example.com";
    let password = "password";
    let name = "authenticate_test_3";

    let resp_body = json!({
        "message": "invalid username and password combination",
        "status": 401,
    });

    let mock = mockito::mock("POST", "/keys")
        .expect(1)
        .match_header("authorization", "Basic bWVAZXhhbXBsZS5jb206cGFzc3dvcmQ=")
        .match_header("content-type", "application/json")
        .match_header("accept", "application/json")
        .match_body(Matcher::Json(json!({
            "name": name,
            "permissions":[{ "domain": "api", "resource": "write" }]
        })))
        .with_status(401)
        .with_body(resp_body.to_string())
        .create();

    let mut client = UnauthenticatedClient::new();
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.authenticate(username, password, name).await {
        Err(AuthenticateError::InvalidCredentials) => (),
        result => panic!(
            "expected Err(AuthenticateError::InvalidCredentials), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn remove_docs_success() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";

    let mock = mockito::mock(
        "DELETE",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(204)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    client
        .remove_docs(package, version)
        .await
        .expect("should be ok");

    mock.assert();
}

#[tokio::test]
async fn remove_docs_unknown_package_version() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib_this_does_not_exist";
    let version = "0.8.0";

    let mock = mockito::mock(
        "DELETE",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(404)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.remove_docs(package, version).await {
        Err(RemoveDocsError::NotFound(p, v)) if p == package && v == version => (),
        result => panic!(
            "expected Err(RemoveDocsError::NotFound(package, version)) got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn remove_docs_rate_limted() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";

    let mock = mockito::mock(
        "DELETE",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(429)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.remove_docs(package, version).await {
        Err(RemoveDocsError::RateLimited) => (),
        result => panic!(
            "expected Err(RemoveDocsError::RateLimited), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn remove_docs_invalid_token() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";

    let mock = mockito::mock(
        "DELETE",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(401)
    .with_body(
        json!({
            "message": "invalid API key",
            "status": 401,
        })
        .to_string(),
    )
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.remove_docs(package, version).await {
        Err(RemoveDocsError::InvalidApiKey) => (),
        result => panic!(
            "expected Err(RemoveDocsError::InvalidApiKey), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn remove_docs_forbidden() {
    let token = "my-api-token-here";
    let package = "jason";
    let version = "1.2.0";

    let mock = mockito::mock(
        "DELETE",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(403)
    .with_body(
        json!({
            "message": "account is not authorized for this action",
            "status": 403,
        })
        .to_string(),
    )
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.remove_docs(package, version).await {
        Err(RemoveDocsError::Forbidden) => (),
        result => panic!("expected Err(RemoveDocsError::Forbidden), got {:?}", result),
    }

    mock.assert();
}

#[tokio::test]
async fn remove_docs_bad_package_name() {
    let token = "my-api-token-here";
    let package = "not valid";
    let version = "1.2.0";

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.remove_docs(package, version).await {
        Err(RemoveDocsError::BadPackage(p, v)) if p == package && v == version => (),
        result => panic!(
            "expected Err(RemoveDocsError::BadPackage), got {:?}",
            result
        ),
    }
}

#[tokio::test]
async fn publish_docs_success() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let mock = mockito::mock(
        "POST",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(201)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.publish_docs(package, version, tarball).await {
        Ok(()) => (),
        result => panic!("expected Ok(()), got {:?}", result),
    }

    mock.assert()
}

#[tokio::test]
async fn publish_docs_bad_package_name() {
    let token = "my-api-token-here";
    let package = "invalid name";
    let version = "0.8.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let client = AuthenticatedClient::new(token.to_string());

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::BadPackage(p, v)) if p == package && v == version => (),
        result => panic!("expected PublishDocsError::BadPackage, got {:?}", result),
    }
}

#[tokio::test]
async fn publish_docs_bad_package_version() {
    let token = "my-api-token-here";
    let package = "name";
    let version = "invalid version";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let client = AuthenticatedClient::new(token.to_string());

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::BadPackage(p, v)) if p == package && v == version => (),
        result => panic!("expected PublishDocsError::BadPackage, got {:?}", result),
    }
}

#[tokio::test]
async fn publish_docs_not_found() {
    let token = "my-api-token-here";
    let package = "name";
    let version = "1.1.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let mock = mockito::mock(
        "POST",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(404)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::NotFound(p, v)) if p == package && v == version => (),
        result => panic!("expected PublishDocsError::NotFound, got {:?}", result),
    }

    mock.assert()
}

#[tokio::test]
async fn publish_docs_rate_limit() {
    let token = "my-api-token-here";
    let package = "name";
    let version = "1.1.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let mock = mockito::mock(
        "POST",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(429)
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::RateLimited) => (),
        result => panic!("expected PublishDocsError::RateLimited, got {:?}", result),
    }

    mock.assert()
}

#[tokio::test]
async fn publish_docs_invalid_api_token() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let mock = mockito::mock(
        "POST",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(401)
    .with_body(
        json!({
            "message": "invalid API key",
            "status": 401,
        })
        .to_string(),
    )
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::InvalidApiKey) => (),
        result => panic!(
            "expected Err(PublishDocsError::InvalidApiKey), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn publish_docs_forbidden() {
    let token = "my-api-token-here";
    let package = "gleam_experimental_stdlib";
    let version = "0.8.0";
    let tarball = Bytes::from_static(std::include_bytes!("../test/example.tar.gz"));

    let mock = mockito::mock(
        "POST",
        format!("/packages/{}/releases/{}/docs", package, version).as_ref(),
    )
    .expect(1)
    .match_header("authorization", token)
    .match_header("accept", "application/json")
    .with_status(403)
    .with_body(
        json!({
            "message": "account is not authorized for this action",
            "status": 403,
        })
        .to_string(),
    )
    .create();

    let mut client = AuthenticatedClient::new(token.to_string());
    client.api_base = url::Url::parse(&mockito::server_url()).unwrap();

    match client.publish_docs(package, version, tarball).await {
        Err(PublishDocsError::Forbidden) => (),
        result => panic!(
            "expected Err(PublishDocsError::Forbidden), got {:?}",
            result
        ),
    }

    mock.assert();
}

#[tokio::test]
async fn get_repository_versions_ok_test() {
    let response_body = std::include_bytes!("../test/versions");

    // Set up test server
    let mock = mockito::mock("GET", "/versions")
        .expect(1)
        .with_status(200)
        .with_body(&response_body[..])
        .create();

    // Test!
    let mut client = UnauthenticatedClient::new();
    client.repository_base = url::Url::parse(&mockito::server_url()).unwrap();

    let versions = client
        .get_repository_versions(std::include_bytes!("../test/public_key"))
        .await;

    let mut expected = HashMap::with_capacity(3);
    expected.insert(
        "one".to_string(),
        vec!["1.0.0".to_string(), "2.0.0".to_string()],
    );
    expected.insert(
        "two".to_string(),
        vec!["1.0.0".to_string(), "1.1.0".to_string()],
    );
    expected.insert(
        "three".to_string(),
        vec!["0.0.0".to_string(), "8.0.0".to_string()],
    );
    assert_eq!(expected, versions.unwrap());

    mock.assert();
}
