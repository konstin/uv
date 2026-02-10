//! Helpers for setting up wiremock-based mock PyPI indexes in integration tests.

use serde_json::{json, Value};
use wiremock::matchers::{basic_auth, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub(crate) const USERNAME: &str = "public";
pub(crate) const PASSWORD: &str = "heron";

/// Path on the mock server where iniconfig wheel is served.
pub(crate) const INICONFIG_WHEEL_PATH: &str =
    "/files/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl";

/// Extract the `host:port` authority from a [`MockServer`].
pub(crate) fn host(server: &MockServer) -> String {
    server
        .uri()
        .strip_prefix("http://")
        .unwrap()
        .to_string()
}

/// Start a [`MockServer`] with authenticated Simple API endpoints and a 401 catch-all.
pub(crate) async fn start_auth_index(packages: &[PackageSimpleApi]) -> MockServer {
    let server = MockServer::start().await;
    mount_auth_index(&server, packages, USERNAME, PASSWORD).await;
    server
}

/// Mount authenticated Simple API endpoints at `/simple/{name}` and a catch-all 401.
pub(crate) async fn mount_auth_index(
    server: &MockServer,
    packages: &[PackageSimpleApi],
    username: &str,
    password: &str,
) {
    mount_packages_with_auth(server, "", packages, username, password).await;
    mount_401_catchall(server).await;
}

/// Mount authenticated Simple API endpoints at `{base_path}/simple/{name}`.
pub(crate) async fn mount_packages_with_auth(
    server: &MockServer,
    base_path: &str,
    packages: &[PackageSimpleApi],
    username: &str,
    password: &str,
) {
    for pkg in packages {
        let body = json!({ "files": pkg.files });

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

    // 404 for authenticated requests to non-existent packages (so uv doesn't
    // treat it as an auth failure from the global 401 catch-all).
    Mock::given(method("GET"))
        .and(path_regex(format!(r"^{base_path}/simple/.+")))
        .and(basic_auth(username, password))
        .respond_with(ResponseTemplate::new(404))
        .with_priority(200)
        .mount(server)
        .await;
}

/// Mount unauthenticated Simple API endpoints at `/simple/{name}`.
pub(crate) async fn mount_index(server: &MockServer, packages: &[PackageSimpleApi]) {
    mount_packages(server, "", packages).await;
}

/// Mount unauthenticated Simple API endpoints at `{base_path}/simple/{name}`.
pub(crate) async fn mount_packages(
    server: &MockServer,
    base_path: &str,
    packages: &[PackageSimpleApi],
) {
    for pkg in packages {
        let body = json!({ "files": pkg.files });

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

/// Mount a catch-all 401 with lowest priority.
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

/// Redirect authenticated `/files/**` requests to real PyPI CDN.
pub(crate) async fn mount_file_redirects_auth(
    server: &MockServer,
    username: &str,
    password: &str,
) {
    Mock::given(method("GET"))
        .and(path_regex(r"^/files/"))
        .and(basic_auth(username, password))
        .respond_with(|req: &wiremock::Request| {
            let cdn_path = req.url.path().strip_prefix("/files").unwrap();
            ResponseTemplate::new(302)
                .insert_header("Location", &format!("https://files.pythonhosted.org{cdn_path}"))
        })
        .mount(server)
        .await;
}

/// Redirect unauthenticated `/files/**` requests to real PyPI CDN.
pub(crate) async fn mount_file_redirects(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path_regex(r"^/files/"))
        .respond_with(|req: &wiremock::Request| {
            let cdn_path = req.url.path().strip_prefix("/files").unwrap();
            ResponseTemplate::new(302)
                .insert_header("Location", &format!("https://files.pythonhosted.org{cdn_path}"))
        })
        .mount(server)
        .await;
}

pub(crate) struct PackageSimpleApi {
    pub(crate) name: &'static str,
    pub(crate) files: Value,
}

/// Pre-built [`PackageSimpleApi`] entries for common test packages.
///
/// `*_local` variants return file URLs pointing to the mock server root
/// (for use with [`mount_file_redirects_auth`] / [`mount_file_redirects`]).
pub(crate) mod packages {
    use serde_json::json;

    pub(crate) fn iniconfig() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "iniconfig",
            files: json!([{
                "filename": "iniconfig-2.0.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl",
                "hashes": { "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374" },
                "requires-python": ">=3.7",
                "size": 5892,
                "upload-time": "2023-01-07T11:08:09.864484Z"
            }, {
                "filename": "iniconfig-2.0.0.tar.gz",
                "url": "https://files.pythonhosted.org/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz",
                "hashes": { "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3" },
                "requires-python": ">=3.7",
                "size": 4646,
                "upload-time": "2023-01-07T11:08:11.254770Z"
            }]),
        }
    }

    /// Iniconfig with file URLs pointing to the mock server root.
    pub(crate) fn iniconfig_local(server_uri: &str) -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "iniconfig",
            files: json!([{
                "filename": "iniconfig-2.0.0-py3-none-any.whl",
                "url": format!("{server_uri}{}", super::INICONFIG_WHEEL_PATH),
                "hashes": { "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374" },
                "requires-python": ">=3.7",
                "size": 5892,
                "upload-time": "2023-01-07T11:08:09.864484Z"
            }, {
                "filename": "iniconfig-2.0.0.tar.gz",
                "url": format!("{server_uri}/files/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz"),
                "hashes": { "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3" },
                "requires-python": ">=3.7",
                "size": 4646,
                "upload-time": "2023-01-07T11:08:11.254770Z"
            }]),
        }
    }

    /// Iniconfig with relative file URLs (`../../../files/packages/...`).
    pub(crate) fn iniconfig_relative() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "iniconfig",
            files: json!([{
                "filename": "iniconfig-2.0.0-py3-none-any.whl",
                "url": format!("../../..{}", super::INICONFIG_WHEEL_PATH),
                "hashes": { "sha256": "b6a85871a79d2e3b22d2d1b94ac2824226a63c6b741c88f7ae975f18b6778374" },
                "requires-python": ">=3.7",
                "size": 5892,
                "upload-time": "2023-01-07T11:08:09.864484Z"
            }, {
                "filename": "iniconfig-2.0.0.tar.gz",
                "url": "../../../files/packages/d7/4b/cbd8e699e64a6f16ca3a8220661b5f83792b3017d0f79807cb8708d33913/iniconfig-2.0.0.tar.gz",
                "hashes": { "sha256": "2d91e135bf72d31a410b17c16da610a82cb55f6b0477d1a902134b24a455b8b3" },
                "requires-python": ">=3.7",
                "size": 4646,
                "upload-time": "2023-01-07T11:08:11.254770Z"
            }]),
        }
    }

    pub(crate) fn anyio() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "anyio",
            files: json!([{
                "filename": "anyio-4.3.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/14/fd/2f20c40b45e4fb4324834aea24bd4afdf1143390242c0b33774da0e2e34f/anyio-4.3.0-py3-none-any.whl",
                "hashes": { "sha256": "048e05d0f6caeed70d731f3db756d35dcc1f35747c8c403364a8332c630441b8" },
                "requires-python": ">=3.8",
                "size": 85584,
                "upload-time": "2024-02-19T08:36:26.842735Z"
            }, {
                "filename": "anyio-4.3.0.tar.gz",
                "url": "https://files.pythonhosted.org/packages/db/4d/3970183622f0330d3c23d9b8a5f52e365e50381fd484d08e3285104333d3/anyio-4.3.0.tar.gz",
                "hashes": { "sha256": "f75253795a87df48568485fd18cdd2a3fa5c4f7c5be8e5e36637733fce06fed6" },
                "requires-python": ">=3.8",
                "size": 159642,
                "upload-time": "2024-02-19T08:36:28.641Z"
            }]),
        }
    }

    pub(crate) fn idna() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "idna",
            files: json!([{
                "filename": "idna-3.6-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/c2/e7/a82b05cf63a603df6e68d59ae6a68bf5064484a0718ea5033660af4b54a9/idna-3.6-py3-none-any.whl",
                "hashes": { "sha256": "c05567e9c24a6b9faaa835c4821bad0590fbb9d5779e7caa6e1cc4978e7eb24f" },
                "requires-python": ">=3.5",
                "size": 61567,
                "upload-time": "2023-11-25T15:40:52.604388Z"
            }, {
                "filename": "idna-3.6.tar.gz",
                "url": "https://files.pythonhosted.org/packages/bf/3f/ea4b9117521a1e9c50344b909be7886dd00a519552724809bb1f486986c2/idna-3.6.tar.gz",
                "hashes": { "sha256": "9ecdbbd083b06798ae1e86adcbfe8ab1479cf864e4ee30fe4e46a003d12491ca" },
                "requires-python": ">=3.5",
                "size": 175426,
                "upload-time": "2023-11-25T15:40:54.902Z"
            }]),
        }
    }

    pub(crate) fn sniffio() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "sniffio",
            files: json!([{
                "filename": "sniffio-1.3.1-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/e9/44/75a9c9421471a6c4805dbf2356f7c181a29c1879239abab1ea2cc8f38b40/sniffio-1.3.1-py3-none-any.whl",
                "hashes": { "sha256": "2f6da418d1f1e0fddd844478f41680e794e6051915791a034ff65e5f100525a2" },
                "requires-python": ">=3.7",
                "size": 10235,
                "upload-time": "2024-02-25T23:20:01.196159Z"
            }, {
                "filename": "sniffio-1.3.1.tar.gz",
                "url": "https://files.pythonhosted.org/packages/a2/87/a6771e1546d97e7e041b6ae58d80074f81b7d5121207425c964ddf5cfdbd/sniffio-1.3.1.tar.gz",
                "hashes": { "sha256": "f4324edc670a0f49750a81b895f35c3adb843cca46f0530f79fc1babb23789dc" },
                "requires-python": ">=3.7",
                "size": 20372,
                "upload-time": "2024-02-25T23:20:04.057Z"
            }]),
        }
    }

    pub(crate) fn typing_extensions() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "typing-extensions",
            files: json!([{
                "filename": "typing_extensions-4.10.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/f9/de/dc04a3ea60b22624b51c703a84bbe0184abcd1d0b9bc8074b5d6b7ab90bb/typing_extensions-4.10.0-py3-none-any.whl",
                "hashes": { "sha256": "69b1a937c3a517342112fb4c6df7e72fc39a38e7891a5730ed4985b5214b5475" },
                "requires-python": ">=3.8",
                "size": 33926,
                "upload-time": "2024-02-25T22:12:47.720766Z"
            }, {
                "filename": "typing_extensions-4.10.0.tar.gz",
                "url": "https://files.pythonhosted.org/packages/16/3a/0d26ce356c7465a19c9ea8814b960f8a36c3b0d07c323176620b7b483e44/typing_extensions-4.10.0.tar.gz",
                "hashes": { "sha256": "b0abd7c89e8fb96f98db18d86106ff1d90ab692004eb746cf6eda2682f91b3cb" },
                "requires-python": ">=3.8",
                "size": 77558,
                "upload-time": "2024-02-25T22:12:49.693Z"
            }]),
        }
    }

    pub(crate) fn executable_application() -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "executable-application",
            files: json!([{
                "filename": "executable_application-0.1.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/fb/f4/df36669d09e0bc54b971592468dc8abf82241babbef4a49af19815be5f45/executable_application-0.1.0-py3-none-any.whl",
                "hashes": { "sha256": "f8bbfeff401011090463bee4c8cd793b66a9c8bef0eb3b8620b98f11fd879090" },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:20:44.929801Z"
            }, {
                "filename": "executable_application-0.2.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/b7/d6/d9e5e20e5fd52a2ff02c4dcd351a2a2fb1e1a159c5de355aded16bccadef/executable_application-0.2.0-py3-none-any.whl",
                "hashes": { "sha256": "2b26f00eb59ebe606697535aee0bfcf1f7ae0dfa7223703cd40cf1c31959d149" },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:21:08.354027Z"
            }, {
                "filename": "executable_application-0.3.0-py3-none-any.whl",
                "url": "https://files.pythonhosted.org/packages/32/97/8ab6fa1bbcb0a888f460c0a19c301f4cc4180573564ad7dd98b5ceca2ab6/executable_application-0.3.0-py3-none-any.whl",
                "hashes": { "sha256": "ca272aee7332e9d266663bc70037cd3ef1d74ffae40030eaf9ca46462dc8dcc6" },
                "requires-python": ">=3.8",
                "upload-time": "2025-01-17T23:21:22.716939Z"
            }]),
        }
    }

    /// Anyio with file URLs pointing to the mock server root.
    pub(crate) fn anyio_local(server_uri: &str) -> super::PackageSimpleApi {
        super::PackageSimpleApi {
            name: "anyio",
            files: json!([{
                "filename": "anyio-4.3.0-py3-none-any.whl",
                "url": format!("{server_uri}/files/packages/14/fd/2f20c40b45e4fb4324834aea24bd4afdf1143390242c0b33774da0e2e34f/anyio-4.3.0-py3-none-any.whl"),
                "hashes": { "sha256": "048e05d0f6caeed70d731f3db756d35dcc1f35747c8c403364a8332c630441b8" },
                "requires-python": ">=3.8",
                "size": 85584,
                "upload-time": "2024-02-19T08:36:26.842735Z"
            }, {
                "filename": "anyio-4.3.0.tar.gz",
                "url": format!("{server_uri}/files/packages/db/4d/3970183622f0330d3c23d9b8a5f52e365e50381fd484d08e3285104333d3/anyio-4.3.0.tar.gz"),
                "hashes": { "sha256": "f75253795a87df48568485fd18cdd2a3fa5c4f7c5be8e5e36637733fce06fed6" },
                "requires-python": ">=3.8",
                "size": 159642,
                "upload-time": "2024-02-19T08:36:28.641Z"
            }]),
        }
    }

    pub(crate) fn anyio_all() -> Vec<super::PackageSimpleApi> {
        vec![anyio(), idna(), sniffio()]
    }
}
