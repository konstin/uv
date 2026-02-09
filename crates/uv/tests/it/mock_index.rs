//! Helpers for setting up wiremock-based mock PyPI indexes in integration tests.
//!
//! These replace the external `pypi-proxy.fly.dev` service that was previously used
//! for testing authenticated index access.

use serde_json::{json, Value};
use wiremock::matchers::{basic_auth, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Well-known test credentials used by the mock index.
pub const USERNAME: &str = "public";
pub const PASSWORD: &str = "heron";

/// Create a [`MockServer`] that serves a Simple API index with basic auth.
///
/// - Requests with valid credentials get a 200 response with the Simple API JSON.
/// - Requests without valid credentials get a 401 response.
///
/// The returned URL is the base URL of the server (e.g., `http://127.0.0.1:PORT`).
/// Use `format!("{}/simple", server.uri())` for the index URL.
pub async fn start_auth_index(packages: &[PackageSimpleApi]) -> MockServer {
    let server = MockServer::start().await;
    mount_auth_index(&server, packages, USERNAME, PASSWORD).await;
    server
}

/// Mount authenticated Simple API endpoints on an existing [`MockServer`].
pub async fn mount_auth_index(
    server: &MockServer,
    packages: &[PackageSimpleApi],
    username: &str,
    password: &str,
) {
    for pkg in packages {
        let body = json!({
            "meta": {"api-version": "1.1"},
            "name": pkg.name,
            "files": pkg.files,
        });

        // Authenticated response (mounted first, but more specific so matches first)
        Mock::given(method("GET"))
            .and(path_regex(format!(
                r"^/simple/{name}/?$",
                name = pkg.name
            )))
            .and(basic_auth(username, password))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(body.to_string(), "application/vnd.pypi.simple.v1+json"),
            )
            .mount(server)
            .await;
    }

    // Catch-all 401 for unauthenticated requests (mounted last, least specific)
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .mount(server)
        .await;
}

/// Mount unauthenticated Simple API endpoints on a [`MockServer`].
pub async fn mount_index(server: &MockServer, packages: &[PackageSimpleApi]) {
    for pkg in packages {
        let body = json!({
            "meta": {"api-version": "1.1"},
            "name": pkg.name,
            "files": pkg.files,
        });

        Mock::given(method("GET"))
            .and(path_regex(format!(
                r"^/simple/{name}/?$",
                name = pkg.name
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(body.to_string(), "application/vnd.pypi.simple.v1+json"),
            )
            .mount(server)
            .await;
    }
}

/// Description of a package in the Simple API.
pub struct PackageSimpleApi {
    pub name: &'static str,
    pub files: Value,
}

/// Pre-built Simple API file entries for common test packages.
/// File URLs point to real PyPI, so no local wheel fixtures are needed.
pub mod packages {
    use serde_json::{json, Value};

    pub fn iniconfig() -> Value {
        json!([{
            "filename": "iniconfig-2.0.0-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl",
            "hashes": {
                "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374"
            },
            "requires-python": ">=3.7",
            "upload-time": "2023-01-07T11:08:09.864484Z"
        }, {
            "filename": "iniconfig-2.0.0.tar.gz",
            "url": "https://files.pythonhosted.org/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz",
            "hashes": {
                "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3"
            },
            "requires-python": ">=3.7",
            "upload-time": "2023-01-07T11:08:11.254770Z"
        }])
    }

    pub fn anyio() -> Value {
        json!([{
            "filename": "anyio-4.3.0-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/14/fd/2f20c40b45e4fb4324834aea24bd4afdf1143390242c0b33774da0e2e34f/anyio-4.3.0-py3-none-any.whl",
            "hashes": {
                "sha256": "048e05d0f6caeed70d731f3db756d35dcc1f35747c8c403364a8332c630441b8"
            },
            "requires-python": ">=3.8",
            "upload-time": "2024-02-19T08:36:26.842735Z"
        }])
    }

    pub fn idna() -> Value {
        json!([{
            "filename": "idna-3.6-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/c2/e7/a82b05cf63a603df6e68d59ae6a68bf5064484a0718ea5033660af4b54a9/idna-3.6-py3-none-any.whl",
            "hashes": {
                "sha256": "c05567e9c24a6b9faaa835c4821bad0590fbb9d5779e7caa6e1cc4978e7eb24f"
            },
            "requires-python": ">=3.5",
            "upload-time": "2023-11-25T15:40:52.604388Z"
        }])
    }

    pub fn sniffio() -> Value {
        json!([{
            "filename": "sniffio-1.3.1-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/e9/44/75a9c9421471a6c4805dbf2356f7c181a29c1879239abab1ea2cc8f38b40/sniffio-1.3.1-py3-none-any.whl",
            "hashes": {
                "sha256": "2f6da418d1f1e0fddd844478f41680e794e6051915791a034ff65e5f100525a2"
            },
            "requires-python": ">=3.7",
            "upload-time": "2024-02-25T23:20:01.196159Z"
        }])
    }

    pub fn typing_extensions() -> Value {
        json!([{
            "filename": "typing_extensions-4.10.0-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/f9/de/dc04a3ea60b22624b51c703a84bbe0184abcd1d0b9bc8074b5d6b7ab90bb/typing_extensions-4.10.0-py3-none-any.whl",
            "hashes": {
                "sha256": "69b1a937c3a517342112fb4c6df7e72fc39a38e7891a5730ed4985b5214b5475"
            },
            "requires-python": ">=3.8",
            "upload-time": "2024-02-25T22:12:47.720766Z"
        }])
    }

    pub fn executable_application() -> Value {
        json!([
            {
                "filename": "executable_application-0.1.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/fb/f4/df36669d09e0bc54b971592468dc8abf82241babbef4a49af19815be5f45/executable_application-0.1.0-py3-none-any.whl",
                "hashes": {
                    "sha256": "f8bbfeff401011090463bee4c8cd793b66a9c8bef0eb3b8620b98f11fd879090"
                },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:20:44.929801Z"
            },
            {
                "filename": "executable_application-0.2.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/b7/d6/d9e5e20e5fd52a2ff02c4dcd351a2a2fb1e1a159c5de355aded16bccadef/executable_application-0.2.0-py3-none-any.whl",
                "hashes": {
                    "sha256": "2b26f00eb59ebe606697535aee0bfcf1f7ae0dfa7223703cd40cf1c31959d149"
                },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:21:08.354027Z"
            },
            {
                "filename": "executable_application-0.3.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/32/97/8ab6fa1bbcb0a888f460c0a19c301f4cc4180573564ad7dd98b5ceca2ab6/executable_application-0.3.0-py3-none-any.whl",
                "hashes": {
                    "sha256": "ca272aee7332e9d266663bc70037cd3ef1d74ffae40030eaf9ca46462dc8dcc6"
                },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:21:22.716939Z"
            }
        ])
    }

    /// All packages needed for `anyio` resolution (anyio + its dependencies).
    pub fn anyio_all() -> Vec<super::PackageSimpleApi> {
        vec![
            super::PackageSimpleApi {
                name: "anyio",
                files: anyio(),
            },
            super::PackageSimpleApi {
                name: "idna",
                files: idna(),
            },
            super::PackageSimpleApi {
                name: "sniffio",
                files: sniffio(),
            },
        ]
    }
}
