//! Helpers for setting up wiremock-based mock PyPI indexes in integration tests.
//!
//! Package metadata is fetched from the real PyPI Simple API at mount time,
//! removing the need for hardcoded response data.

use serde_json::Value;
use wiremock::matchers::{basic_auth, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Files uploaded after this cutoff are excluded from proxied responses,
/// keeping mock data stable across new PyPI releases. Matches the default
/// test-framework `EXCLUDE_NEWER` in `uv-test`.
const EXCLUDE_NEWER: &str = "2024-03-25T00:00:00Z";

pub(crate) const USERNAME: &str = "public";
pub(crate) const PASSWORD: &str = "heron";

/// Path on the mock server where iniconfig wheel is served.
pub(crate) const INICONFIG_WHEEL_PATH: &str =
    "/files/packages/ef/a6/62565a6e1cf69e10f5727360368e451d4b7f58beeac6173dc9db836a5b46/iniconfig-2.0.0-py3-none-any.whl";

/// Anyio and its transitive dependencies (for Python >= 3.11).
pub(crate) const ANYIO_PACKAGES: &[&str] = &["anyio", "idna", "sniffio"];

/// Extract the `host:port` authority from a [`MockServer`].
pub(crate) fn host(server: &MockServer) -> String {
    server
        .uri()
        .strip_prefix("http://")
        .expect("server URI should start with http://")
        .to_string()
}

/// Fetch the Simple API JSON response for a package from PyPI,
/// filtering out files uploaded after `exclude_newer`.
async fn fetch_simple_api(client: &reqwest::Client, name: &str, exclude_newer: &str) -> String {
    let url = format!("https://pypi.org/simple/{name}/");
    let text = client
        .get(&url)
        .header("Accept", "application/vnd.pypi.simple.v1+json")
        .send()
        .await
        .unwrap_or_else(|e| panic!("failed to fetch {url}: {e}"))
        .error_for_status()
        .unwrap_or_else(|e| panic!("PyPI returned error for {url}: {e}"))
        .text()
        .await
        .unwrap_or_else(|e| panic!("failed to read response body for {url}: {e}"));

    let mut json: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("invalid JSON from {url}: {e}"));
    if let Some(files) = json.get_mut("files").and_then(Value::as_array_mut) {
        files.retain(|file| {
            file.get("upload-time")
                .and_then(Value::as_str)
                .map_or(true, |t| t < exclude_newer)
        });
    }
    json.to_string()
}

/// Core implementation for mounting Simple API mocks.
async fn mount_simple_responses(
    server: &MockServer,
    base_path: &str,
    packages: &[&str],
    auth: Option<(&str, &str)>,
    exclude_newer: &str,
    transform_body: impl Fn(String) -> String,
) {
    let client = reqwest::Client::new();
    for name in packages {
        let body = transform_body(fetch_simple_api(&client, name, exclude_newer).await);

        let mock =
            Mock::given(method("GET")).and(path_regex(format!(r"^{base_path}/simple/{name}/?$")));
        let mock = if let Some((username, password)) = auth {
            mock.and(basic_auth(username, password))
        } else {
            mock
        };
        mock.respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body, "application/vnd.pypi.simple.v1+json"),
        )
        .mount(server)
        .await;
    }

    if let Some((username, password)) = auth {
        Mock::given(method("GET"))
            .and(path_regex(format!(r"^{base_path}/simple/.+")))
            .and(basic_auth(username, password))
            .respond_with(ResponseTemplate::new(404))
            .with_priority(200)
            .mount(server)
            .await;
    }
}

/// Start a [`MockServer`] with authenticated Simple API endpoints and a 401 catch-all.
pub(crate) async fn start_auth_index(packages: &[&str]) -> MockServer {
    let server = MockServer::start().await;
    mount_auth_index(&server, packages, USERNAME, PASSWORD).await;
    server
}

/// Like [`start_auth_index`], but with a custom upload-time cutoff for packages
/// whose releases are newer than the default [`EXCLUDE_NEWER`].
pub(crate) async fn start_auth_index_with_exclude_newer(
    packages: &[&str],
    exclude_newer: &str,
) -> MockServer {
    let server = MockServer::start().await;
    mount_simple_responses(
        &server,
        "",
        packages,
        Some((USERNAME, PASSWORD)),
        exclude_newer,
        |body| body,
    )
    .await;
    mount_401_catchall(&server).await;
    server
}

/// Mount authenticated Simple API endpoints at `/simple/{name}` and a catch-all 401.
pub(crate) async fn mount_auth_index(
    server: &MockServer,
    packages: &[&str],
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
    packages: &[&str],
    username: &str,
    password: &str,
) {
    mount_simple_responses(
        server,
        base_path,
        packages,
        Some((username, password)),
        EXCLUDE_NEWER,
        |body| body,
    )
    .await;
}

/// Like [`mount_packages_with_auth`], but rewrites file URLs to point at the mock server.
pub(crate) async fn mount_packages_local_with_auth(
    server: &MockServer,
    base_path: &str,
    packages: &[&str],
    username: &str,
    password: &str,
) {
    let server_uri = server.uri();
    mount_simple_responses(
        server,
        base_path,
        packages,
        Some((username, password)),
        EXCLUDE_NEWER,
        |body| {
            body.replace(
                "https://files.pythonhosted.org",
                &format!("{server_uri}/files"),
            )
        },
    )
    .await;
}

/// Mount unauthenticated Simple API endpoints at `/simple/{name}`.
pub(crate) async fn mount_index(server: &MockServer, packages: &[&str]) {
    mount_packages(server, "", packages).await;
}

/// Mount unauthenticated Simple API endpoints at `{base_path}/simple/{name}`.
pub(crate) async fn mount_packages(
    server: &MockServer,
    base_path: &str,
    packages: &[&str],
) {
    mount_simple_responses(server, base_path, packages, None, EXCLUDE_NEWER, |body| body).await;
}

/// Like [`mount_packages`], but rewrites file URLs to relative paths.
pub(crate) async fn mount_packages_relative(
    server: &MockServer,
    base_path: &str,
    packages: &[&str],
) {
    let depth = base_path.split('/').filter(|s| !s.is_empty()).count() + 2;
    let prefix = "../".repeat(depth);
    mount_simple_responses(server, base_path, packages, None, EXCLUDE_NEWER, |body| {
        body.replace(
            "https://files.pythonhosted.org/",
            &format!("{prefix}files/"),
        )
    })
    .await;
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
            let cdn_path = req.url.path().strip_prefix("/files").expect("/files prefix");
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
            let cdn_path = req.url.path().strip_prefix("/files").expect("/files prefix");
            ResponseTemplate::new(302)
                .insert_header("Location", &format!("https://files.pythonhosted.org{cdn_path}"))
        })
        .mount(server)
        .await;
}
