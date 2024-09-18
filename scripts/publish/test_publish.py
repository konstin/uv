# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "httpx>=0.27,<0.28",
#     "packaging>=24.1,<25",
# ]
# ///

"""
Test `uv publish` by uploading a new version of astral-test-<test case> to testpypi,
authenticating by one of various options.

# Setup

**astral-test-token**
Set the `UV_TEST_PUBLISH_TOKEN` environment variables.

**astral-test-password**
Set the `UV_TEST_PUBLISH_PASSWORD` environment variable.
This project also uses token authentication since it's the only thing that PyPI
supports, but they both CLI options.
TODO(konsti): Add an index for testing that supports username/password.

**astral-test-keyring**
```console
uv pip install keyring
keyring set https://test.pypi.org/legacy/?astral-test-keyring __token__
```
The query parameter a horrible hack stolen from
https://github.com/pypa/twine/issues/565#issue-555219267
to prevent the other projects from implicitly using the same credentials.

**astral-test-trusted-publishing**
This one only works in GitHub Actions on astral-sh/uv in `ci.yml` - sorry!
"""

import os
import re
from argparse import ArgumentParser
from pathlib import Path
from shutil import rmtree
from subprocess import check_call

import httpx
from packaging.utils import parse_sdist_filename, parse_wheel_filename

cwd = Path(__file__).parent


def get_new_version(project_name: str) -> str:
    """Return the next free path version on pypi"""
    index_page = f"https://test.pypi.org/simple/{project_name}/?format=application/vnd.pypi.simple.v1+json"
    data = httpx.get(index_page).json()
    versions = set()
    for file in data["files"]:
        if file["filename"].endswith(".whl"):
            [_name, version, _build, _tags] = parse_wheel_filename(file["filename"])
        else:
            [_name, version] = parse_sdist_filename(file["filename"])
        versions.add(version)
    max_version = max(versions)

    # Bump the path version to obtain an empty version
    release = list(max_version.release)
    release[-1] += 1
    return ".".join(str(i) for i in release)


def create_project(project_name: str, uv: Path):
    if cwd.joinpath(project_name).exists():
        rmtree(cwd.joinpath(project_name))
    check_call([uv, "init", "--lib", project_name], cwd=cwd)
    pyproject_toml = cwd.joinpath(project_name).joinpath("pyproject.toml")

    # Set to an unclaimed version
    toml = pyproject_toml.read_text()
    new_version = get_new_version(project_name)
    toml = re.sub('version = ".*"', f'version = "{new_version}"', toml)
    pyproject_toml.write_text(toml)


def publish_project(project_name: str, uv: Path):
    # Create the project
    create_project(project_name, uv)

    # Build the project
    check_call([uv, "build"], cwd=cwd.joinpath(project_name))

    # Upload the project
    if project_name == "astral-test-token":
        env = os.environ.copy()
        env["UV_PUBLISH_TOKEN"] = os.environ["UV_TEST_PUBLISH_TOKEN"]
        check_call(
            [
                uv,
                "publish",
                "--publish-url",
                "https://test.pypi.org/legacy/",
            ],
            cwd=cwd.joinpath(project_name),
            env=env,
        )
    elif project_name == "astral-test-password":
        env = os.environ.copy()
        env["UV_PUBLISH_PASSWORD"] = os.environ["UV_TEST_PUBLISH_PASSWORD"]
        check_call(
            [
                uv,
                "publish",
                "--publish-url",
                "https://test.pypi.org/legacy/",
                "--username",
                "__token__",
            ],
            cwd=cwd.joinpath(project_name),
            env=env,
        )
    elif project_name == "astral-test-keyring":
        check_call(
            [
                uv,
                "publish",
                "--publish-url",
                "https://test.pypi.org/legacy/?astral-test-keyring",
                "--username",
                "__token__",
                "--keyring-provider",
                "subprocess",
            ],
            cwd=cwd.joinpath(project_name),
        )
    elif project_name == "astral-test-trusted-publishing":
        check_call(
            [
                uv,
                "publish",
                "--publish-url",
                "https://test.pypi.org/legacy/",
                "--trusted-publishing",
            ],
            cwd=cwd.joinpath(project_name),
        )
    else:
        raise ValueError(f"Unknown project name: {project_name}")


def main():
    parser = ArgumentParser()
    projects = [
        "astral-test-token",
        "astral-test-password",
        "astral-test-keyring",
        "astral-test-trusted-publishing",
    ]
    parser.add_argument("projects", choices=projects + ["all"], nargs="+")
    parser.add_argument("--uv")
    args = parser.parse_args()

    if args.uv:
        # We change the working directory for the subprocess calls, so we have to
        # absolutize the path.
        uv = Path.cwd().joinpath(args.uv)
    else:
        check_call(["cargo", "build"])
        executable_suffix = ".exe" if os.name == "nt" else ""
        uv = cwd.parent.parent.joinpath(f"target/debug/uv{executable_suffix}")

    if args.projects == ["all"]:
        projects = projects
    else:
        projects = args.projects

    for project_name in projects:
        publish_project(project_name, uv)


if __name__ == "__main__":
    main()
