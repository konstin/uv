//! Helpers for setting up wiremock-based mock PyPI indexes in integration tests.
//!
//! These replace the external `pypi-proxy.fly.dev` service that was previously used
//! for testing authenticated index access.

use serde_json::{json, Value};
use wiremock::matchers::{basic_auth, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Well-known test credentials used by the mock index.
pub(crate) const USERNAME: &str = "public";
pub(crate) const PASSWORD: &str = "heron";

/// Real PyPI CDN URL for iniconfig wheel.
const INICONFIG_WHEEL_URL: &str = "https://files.pythonhosted.org/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl";
/// Real PyPI CDN URL for iniconfig sdist.
const INICONFIG_SDIST_URL: &str = "https://files.pythonhosted.org/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz";

/// Path on the mock server where iniconfig wheel is served.
pub(crate) const INICONFIG_WHEEL_PATH: &str =
    "/files/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl";
/// Path on the mock server where iniconfig sdist is served.
pub(crate) const INICONFIG_SDIST_PATH: &str =
    "/files/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz";

/// Create a [`MockServer`] that serves a Simple API index with basic auth.
///
/// - Requests with valid credentials get a 200 response with the Simple API JSON.
/// - Requests without valid credentials get a 401 response.
///
/// The returned URL is the base URL of the server (e.g., `http://127.0.0.1:PORT`).
/// Use `format!("{}/simple", server.uri())` for the index URL.
pub(crate) async fn start_auth_index(packages: &[PackageSimpleApi]) -> MockServer {
    let server = MockServer::start().await;
    mount_auth_index(&server, packages, USERNAME, PASSWORD).await;
    server
}

/// Mount authenticated Simple API endpoints on an existing [`MockServer`].
///
/// Mounts package endpoints at `/simple/{name}` and a catch-all 401.
pub(crate) async fn mount_auth_index(
    server: &MockServer,
    packages: &[PackageSimpleApi],
    username: &str,
    password: &str,
) {
    mount_packages_with_auth(server, "", packages, username, password).await;
    mount_401_catchall(server).await;
}

/// Mount authenticated Simple API endpoints at a path prefix.
///
/// For example, `base_path = "/basic-auth"` mounts at `/basic-auth/simple/{name}`.
/// An empty string mounts at `/simple/{name}`.
pub(crate) async fn mount_packages_with_auth(
    server: &MockServer,
    base_path: &str,
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

        Mock::given(method("GET"))
            .and(path_regex(format!(
                r"^{base_path}/simple/{name}/?$",
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

    // Catch-all for authenticated requests to non-existent packages on this index.
    // Returns 404 so uv knows the package doesn't exist (rather than falling through
    // to the global 401 catch-all which would be treated as an auth failure).
    Mock::given(method("GET"))
        .and(path_regex(format!(r"^{base_path}/simple/.+")))
        .and(basic_auth(username, password))
        .respond_with(ResponseTemplate::new(404))
        .with_priority(200)
        .mount(server)
        .await;
}

/// Mount unauthenticated Simple API endpoints on a [`MockServer`].
///
/// Mounts package endpoints at `/simple/{name}`.
pub(crate) async fn mount_index(server: &MockServer, packages: &[PackageSimpleApi]) {
    mount_packages(server, "", packages).await;
}

/// Mount unauthenticated Simple API endpoints at a path prefix.
///
/// For example, `base_path = "/relative"` mounts at `/relative/simple/{name}`.
/// An empty string mounts at `/simple/{name}`.
pub(crate) async fn mount_packages(
    server: &MockServer,
    base_path: &str,
    packages: &[PackageSimpleApi],
) {
    for pkg in packages {
        let body = json!({
            "meta": {"api-version": "1.1"},
            "name": pkg.name,
            "files": pkg.files,
        });

        Mock::given(method("GET"))
            .and(path_regex(format!(
                r"^{base_path}/simple/{name}/?$",
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

/// Mount a catch-all 401 response with `WWW-Authenticate` header.
///
/// Uses lowest priority so that more-specific mocks (e.g. file endpoints
/// mounted later) take precedence.
pub(crate) async fn mount_401_catchall(server: &MockServer) {
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("WWW-Authenticate", "Basic realm=\"test\""),
        )
        .with_priority(u8::MAX)
        .mount(server)
        .await;
}

/// Mount authenticated file-serving endpoints for iniconfig that redirect to real PyPI CDN.
///
/// Authenticated requests get a 302 redirect to the real file URL.
/// Unauthenticated requests hit the catch-all 401 from [`mount_401_catchall`].
pub(crate) async fn mount_iniconfig_files_auth(
    server: &MockServer,
    username: &str,
    password: &str,
) {
    Mock::given(method("GET"))
        .and(path(INICONFIG_WHEEL_PATH))
        .and(basic_auth(username, password))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", INICONFIG_WHEEL_URL),
        )
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path(INICONFIG_SDIST_PATH))
        .and(basic_auth(username, password))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", INICONFIG_SDIST_URL),
        )
        .mount(server)
        .await;
}

/// Mount unauthenticated file-serving endpoints for iniconfig that redirect to real PyPI CDN.
pub(crate) async fn mount_iniconfig_files(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path(INICONFIG_WHEEL_PATH))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", INICONFIG_WHEEL_URL),
        )
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path(INICONFIG_SDIST_PATH))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", INICONFIG_SDIST_URL),
        )
        .mount(server)
        .await;
}

/// Description of a package in the Simple API.
pub(crate) struct PackageSimpleApi {
    pub(crate) name: &'static str,
    pub(crate) files: Value,
}

/// Pre-built Simple API file entries for common test packages.
///
/// Functions named `*_local` return file URLs relative to the mock server root,
/// for use with [`mount_iniconfig_files_auth`] / [`mount_iniconfig_files`].
///
/// Other functions return file URLs pointing to real PyPI CDN.
pub(crate) mod packages {
    use serde_json::{json, Value};

    /// Iniconfig with file URLs pointing to real PyPI CDN.
    pub(crate) fn iniconfig() -> Value {
        json!([{
            "filename": "iniconfig-2.0.0-py3-none-any.whl",
            "url": "https://files.pythonhosted.org/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl",
            "hashes": {
                "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374"
            },
            "requires-python": ">=3.7",
            "size": 5892,
            "upload-time": "2023-01-07T11:08:09.864484Z"
        }, {
            "filename": "iniconfig-2.0.0.tar.gz",
            "url": "https://files.pythonhosted.org/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz",
            "hashes": {
                "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3"
            },
            "requires-python": ">=3.7",
            "size": 4646,
            "upload-time": "2023-01-07T11:08:11.254770Z"
        }])
    }

    /// Iniconfig with file URLs relative to the mock server root.
    ///
    /// Use this with [`super::mount_iniconfig_files_auth`] or [`super::mount_iniconfig_files`]
    /// when tests need to verify file download authentication behavior.
    pub(crate) fn iniconfig_local(server_uri: &str) -> Value {
        json!([{
            "filename": "iniconfig-2.0.0-py3-none-any.whl",
            "url": format!("{server_uri}{}", super::INICONFIG_WHEEL_PATH),
            "hashes": {
                "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374"
            },
            "requires-python": ">=3.7",
            "size": 5892,
            "upload-time": "2023-01-07T11:08:09.864484Z"
        }, {
            "filename": "iniconfig-2.0.0.tar.gz",
            "url": format!("{server_uri}{}", super::INICONFIG_SDIST_PATH),
            "hashes": {
                "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3"
            },
            "requires-python": ">=3.7",
            "size": 4646,
            "upload-time": "2023-01-07T11:08:11.254770Z"
        }])
    }

    /// Iniconfig with relative file URLs (for testing relative link resolution).
    ///
    /// URLs are relative to the index page URL. When the index is at
    /// `/relative/simple/iniconfig/`, we need `../../../files/packages/...`
    /// to resolve to `/files/packages/...` (up 3 levels: iniconfig → simple → relative → /).
    pub(crate) fn iniconfig_relative() -> Value {
        json!([{
            "filename": "iniconfig-2.0.0-py3-none-any.whl",
            "url": format!("../../..{}", super::INICONFIG_WHEEL_PATH),
            "hashes": {
                "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374"
            },
            "requires-python": ">=3.7",
            "size": 5892,
            "upload-time": "2023-01-07T11:08:09.864484Z"
        }, {
            "filename": "iniconfig-2.0.0.tar.gz",
            "url": format!("../../..{}", super::INICONFIG_SDIST_PATH),
            "hashes": {
                "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3"
            },
            "requires-python": ">=3.7",
            "size": 4646,
            "upload-time": "2023-01-07T11:08:11.254770Z"
        }])
    }

    pub(crate) fn anyio() -> Value {
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

    pub(crate) fn idna() -> Value {
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

    pub(crate) fn sniffio() -> Value {
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

    pub(crate) fn typing_extensions() -> Value {
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

    pub(crate) fn executable_application() -> Value {
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
    pub(crate) fn anyio_all() -> Vec<super::PackageSimpleApi> {
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
