mod proto;

#[cfg(test)]
mod tests;

use crate::proto::{signed::Signed, versions::Versions};
use async_trait::async_trait;
use bytes::buf::Buf;
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;

#[async_trait]
pub trait Client {
    fn http_client(&self) -> reqwest::Client;
    fn api_base_url(&self) -> &url::Url;
    fn repository_base_url(&self) -> &url::Url;

    /// Authenticate with the Hex API using a username and password in order
    /// to get an API token, enabling accessing of more APIs and raising the
    /// rate limit.
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
        token_name: &str,
    ) -> Result<AuthenticatedClient, AuthenticateError> {
        let body = json!({
            "name": token_name,
            "permissions": [{
                "domain": "api",
                "resource": "write",
            }],
        });

        let response = self
            .http_client()
            .post(self.api_base_url().join("keys").unwrap())
            .basic_auth(username, Some(password))
            .json(&body)
            .send()
            .await
            .map_err(AuthenticateError::Http)?;

        match response.status() {
            StatusCode::CREATED => {
                let body: AuthenticateResponseCreated =
                    response.json().await.map_err(AuthenticateError::Http)?;
                Ok(AuthenticatedClient {
                    repository_base: self.repository_base_url().clone(),
                    api_base: self.api_base_url().clone(),
                    api_token: body.secret,
                })
            }

            StatusCode::TOO_MANY_REQUESTS => Err(AuthenticateError::RateLimited),

            StatusCode::UNAUTHORIZED => Err(AuthenticateError::InvalidCredentials),

            status => Err(AuthenticateError::UnexpectedResponse(
                status,
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// Get the names and versions of all of the packages on the package registry.
    ///
    async fn get_repository_versions(
        &self,
        public_key: &[u8],
    ) -> Result<HashMap<String, Vec<String>>, GetRepositoryVersionsError> {
        let response = self
            .http_client()
            .get(self.repository_base_url().join("versions").unwrap())
            .send()
            .await
            .map_err(GetRepositoryVersionsError::Http)?;

        match response.status() {
            StatusCode::OK => (),
            status => {
                return Err(GetRepositoryVersionsError::UnexpectedResponse(
                    status,
                    response.text().await.unwrap_or_default(),
                ));
            }
        };

        let body = response
            .bytes()
            .await
            .map_err(GetRepositoryVersionsError::Http)?
            .reader();

        let mut body = GzDecoder::new(body);
        let signed = protobuf::parse_from_reader::<Signed>(&mut body)
            .map_err(GetRepositoryVersionsError::DecodeFailed)?;

        let payload = verify_payload(signed, public_key)
            .map_err(|_| GetRepositoryVersionsError::IncorrectPayloadSignature)?;

        let versions = protobuf::parse_from_bytes::<Versions>(&payload)
            .map_err(GetRepositoryVersionsError::DecodeFailed)?
            .take_packages()
            .into_iter()
            .map(|mut n| (n.take_name(), n.take_versions().into_vec()))
            .collect();

        Ok(versions)
    }

    /// Get the information for a package in the repository.
    ///
    async fn get_package(&self, name: &str, public_key: &[u8]) -> Result<Package, GetPackageError> {
        let response = self
            .http_client()
            .get(
                self.repository_base_url()
                    .join(&format!("packages/{}", name))
                    .unwrap(),
            )
            .send()
            .await
            .map_err(GetPackageError::Http)?;

        match response.status() {
            StatusCode::OK => (),
            StatusCode::NOT_FOUND => return Err(GetPackageError::NotFound),
            status => {
                return Err(GetPackageError::UnexpectedResponse(
                    status,
                    response.text().await.unwrap_or_default(),
                ));
            }
        };

        let body = response
            .bytes()
            .await
            .map_err(GetPackageError::Http)?
            .reader();

        let mut body = GzDecoder::new(body);
        let signed = protobuf::parse_from_reader::<Signed>(&mut body)
            .map_err(GetPackageError::DecodeFailed)?;

        let payload = verify_payload(signed, public_key)
            .map_err(|_| GetPackageError::IncorrectPayloadSignature)?;

        let mut package = protobuf::parse_from_bytes::<proto::package::Package>(&payload)
            .map_err(GetPackageError::DecodeFailed)?;

        let package = Package {
            name: package.take_name(),
            repository: package.take_repository(),
            releases: package
                .take_releases()
                .into_iter()
                .map(proto_to_release)
                .collect(),
        };

        Ok(package)
    }
}

fn proto_to_retirement_status(
    mut status: proto::package::RetirementStatus,
) -> Option<RetirementStatus> {
    if status.has_reason() {
        Some(RetirementStatus {
            message: status.take_message(),
            reason: proto_to_retirement_reason(status.get_reason()),
        })
    } else {
        None
    }
}

fn proto_to_retirement_reason(reason: proto::package::RetirementReason) -> RetirementReason {
    use proto::package::RetirementReason::*;
    match reason {
        RETIRED_OTHER => RetirementReason::Other,
        RETIRED_INVALID => RetirementReason::Invalid,
        RETIRED_SECURITY => RetirementReason::Security,
        RETIRED_DEPRECATED => RetirementReason::Deprecated,
        RETIRED_RENAMED => RetirementReason::Renamed,
    }
}

fn proto_to_dep(mut dep: proto::package::Dependency) -> Dependency {
    let app = if dep.has_app() {
        Some(dep.take_app())
    } else {
        None
    };
    let repository = if dep.has_repository() {
        Some(dep.take_repository())
    } else {
        None
    };
    Dependency {
        package: dep.take_package(),
        requirement: dep.take_requirement(),
        optional: dep.has_optional(),
        app,
        repository,
    }
}

fn proto_to_release(mut release: proto::package::Release) -> Release {
    Release {
        version: release.take_version(),
        outer_checksum: release.take_outer_checksum(),
        retirement_status: proto_to_retirement_status(release.take_retired()),
        dependencies: release
            .take_dependencies()
            .into_iter()
            .map(proto_to_dep)
            .collect(),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub repository: String,
    pub releases: Vec<Release>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Release {
    /// Release version
    pub version: String,
    /// All dependencies of the release
    pub dependencies: Vec<Dependency>,
    /// If set the release is retired, a retired release should only be
    /// resolved if it has already been locked in a project
    pub retirement_status: Option<RetirementStatus>,
    /// sha256 checksum of outer package tarball
    /// required when encoding but optional when decoding
    pub outer_checksum: Vec<u8>,
}

impl Release {
    pub fn is_retired(&self) -> bool {
        self.retirement_status.is_some()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RetirementStatus {
    pub reason: RetirementReason,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RetirementReason {
    Other,
    Invalid,
    Security,
    Deprecated,
    Renamed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    /// Package name of dependency
    pub package: String,
    /// Version requirement of dependency
    pub requirement: String,
    /// If true the package is optional and does not need to be resolved
    /// unless another package has specified it as a non-optional dependency.
    pub optional: bool,
    /// If set is the OTP application name of the dependency, if not set the
    /// application name is the same as the package name
    pub app: Option<String>,
    /// If set, the repository where the dependency is located
    pub repository: Option<String>,
}

#[derive(Error, Debug)]
pub enum GetPackageError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("an unexpected response was sent by Hex")]
    UnexpectedResponse(StatusCode, String),

    #[error("the payload signature does not match the downloaded payload")]
    IncorrectPayloadSignature,

    #[error("no package was found in the repository with the given name")]
    NotFound,

    #[error(transparent)]
    DecodeFailed(#[from] protobuf::ProtobufError),
}

#[derive(Error, Debug)]
pub enum GetRepositoryVersionsError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("an unexpected response was sent by Hex")]
    UnexpectedResponse(StatusCode, String),

    #[error("the payload signature does not match the downloaded payload")]
    IncorrectPayloadSignature,

    #[error(transparent)]
    DecodeFailed(#[from] protobuf::ProtobufError),
}

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), " (", env!("CARGO_PKG_VERSION"), ")");

#[derive(Debug)]
pub struct UnauthenticatedClient {
    pub api_base: url::Url,
    pub repository_base: url::Url,
}

impl Client for UnauthenticatedClient {
    fn http_client(&self) -> reqwest::Client {
        let mut headers = http::header::HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());

        reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()
            .expect("failed to build API client")
    }

    fn api_base_url(&self) -> &url::Url {
        &self.api_base
    }

    fn repository_base_url(&self) -> &url::Url {
        &self.repository_base
    }
}

impl UnauthenticatedClient {
    pub fn new() -> Self {
        Self {
            api_base: url::Url::parse("https://hex.pm/api/").unwrap(),
            repository_base: url::Url::parse("https://repo.hex.pm/").unwrap(),
        }
    }
}

#[derive(Error, Debug)]
pub enum AuthenticateError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("the rate limit for the Hex API has been exceeded for this IP")]
    RateLimited,

    #[error("invalid username and password combination")]
    InvalidCredentials,

    #[error("an unexpected response was sent by Hex")]
    UnexpectedResponse(StatusCode, String),
}

#[derive(Debug, Deserialize)]
struct AuthenticateResponseCreated {
    secret: String,
}

#[derive(Debug)]
pub struct AuthenticatedClient {
    pub api_base: url::Url,
    pub repository_base: url::Url,
    pub api_token: String,
}

impl Client for AuthenticatedClient {
    fn http_client(&self) -> reqwest::Client {
        let mut headers = http::header::HeaderMap::new();
        headers.insert("Authorization", self.api_token.parse().unwrap());
        headers.insert("Accept", "application/json".parse().unwrap());

        reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()
            .expect("failed to build API client")
    }

    fn api_base_url(&self) -> &url::Url {
        &self.api_base
    }

    fn repository_base_url(&self) -> &url::Url {
        &self.repository_base
    }
}

impl AuthenticatedClient {
    pub fn new(api_token: String) -> Self {
        Self {
            api_base: url::Url::parse("https://hex.pm/api/").unwrap(),
            repository_base: url::Url::parse("https://repo.hex.pm/").unwrap(),
            api_token,
        }
    }

    pub async fn remove_docs<'a>(
        &self,
        package_name: &'a str,
        version: &'a str,
    ) -> Result<(), RemoveDocsError<'a>> {
        validate_package_and_version(package_name, version)
            .map_err(|_| RemoveDocsError::BadPackage(package_name, version))?;

        let url = self
            .api_base
            .join(format!("packages/{}/releases/{}/docs", package_name, version).as_str())
            .expect("building remove_docs url");

        let response = self
            .http_client()
            .delete(url.to_string().as_str())
            .send()
            .await
            .map_err(RemoveDocsError::Http)?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(RemoveDocsError::NotFound(package_name, version)),
            StatusCode::TOO_MANY_REQUESTS => Err(RemoveDocsError::RateLimited),
            StatusCode::UNAUTHORIZED => Err(RemoveDocsError::InvalidApiKey),
            StatusCode::FORBIDDEN => Err(RemoveDocsError::Forbidden),
            status => Err(RemoveDocsError::UnexpectedResponse(
                status,
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    pub async fn publish_docs<'a>(
        &self,
        package_name: &'a str,
        version: &'a str,
        gzipped_tarball: bytes::Bytes,
    ) -> Result<(), PublishDocsError<'a>> {
        validate_package_and_version(package_name, version)
            .map_err(|_| PublishDocsError::BadPackage(package_name, version))?;

        let url = self
            .api_base
            .join(format!("packages/{}/releases/{}/docs", package_name, version).as_str())
            .expect("building publish_docs url");

        let response = self
            .http_client()
            .post(url.to_string().as_str())
            .body(gzipped_tarball)
            .send()
            .await
            .map_err(PublishDocsError::Http)?;

        match response.status() {
            StatusCode::CREATED => Ok(()),
            StatusCode::NOT_FOUND => Err(PublishDocsError::NotFound(package_name, version)),
            StatusCode::TOO_MANY_REQUESTS => Err(PublishDocsError::RateLimited),
            StatusCode::UNAUTHORIZED => Err(PublishDocsError::InvalidApiKey),
            StatusCode::FORBIDDEN => Err(PublishDocsError::Forbidden),
            status => Err(PublishDocsError::UnexpectedResponse(
                status,
                response.text().await.unwrap_or_default(),
            )),
        }
    }
}

#[derive(Error, Debug)]
pub enum RemoveDocsError<'a> {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("the given package name and version {0} {1} are not valid")]
    BadPackage(&'a str, &'a str),

    #[error("could not find package {0} with version {1}")]
    NotFound(&'a str, &'a str),

    #[error("the rate limit for the Hex API has been exceeded for this IP")]
    RateLimited,

    #[error("the given API key was not valid")]
    InvalidApiKey,

    #[error("this account is not authorized for this action")]
    Forbidden,

    #[error("an unexpected response was sent by Hex")]
    UnexpectedResponse(StatusCode, String),
}

#[derive(Error, Debug)]
pub enum PublishDocsError<'a> {
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("the given package name and version {0} {1} are not valid")]
    BadPackage(&'a str, &'a str),

    #[error("could not find package {0} with version {1}")]
    NotFound(&'a str, &'a str),

    #[error("the rate limit for the Hex API has been exceeded for this IP")]
    RateLimited,

    #[error("the given API key was not valid")]
    InvalidApiKey,

    #[error("this account is not authorized for this action")]
    Forbidden,

    #[error("an unexpected response was sent by Hex")]
    UnexpectedResponse(StatusCode, String),
}

fn validate_package_and_version(package: &str, version: &str) -> Result<(), ()> {
    lazy_static! {
        static ref PACKAGE_PATTERN: Regex = Regex::new(r#"^[a-z_-]+$"#).unwrap();
        static ref VERSION_PATTERN: Regex = Regex::new(r#"^[a-zA-Z-0-9\._-]+$"#).unwrap();
    }
    if !PACKAGE_PATTERN.is_match(package) {
        return Err(());
    }
    if !VERSION_PATTERN.is_match(version) {
        return Err(());
    }
    Ok(())
}

// To quote the docs:
//
// > All resources will be signed by the repository's private key.
// > A signed resource is wrapped in a Signed message. The data under
// > the payload field is signed by the signature field.
// >
// > The signature is an (unencoded) RSA signature of the (unencoded)
// > SHA-512 digest of the payload.
//
// https://github.com/hexpm/specifications/blob/master/registry-v2.md#signing
//
fn verify_payload(mut signed: Signed, pem_public_key: &[u8]) -> Result<Vec<u8>, ()> {
    let (_, pem) = x509_parser::pem::pem_to_der(pem_public_key).map_err(|_| ())?;
    let (_, spki) = x509_parser::parse_subject_public_key_info(&pem.contents).map_err(|_| ())?;
    let payload = signed.take_payload();
    let verification = ring::signature::UnparsedPublicKey::new(
        &ring::signature::RSA_PKCS1_2048_8192_SHA512,
        &spki.subject_public_key,
    )
    .verify(payload.as_slice(), signed.get_signature());

    if verification.is_ok() {
        Ok(payload)
    } else {
        Err(())
    }
}
