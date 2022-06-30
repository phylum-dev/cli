use std::fs;
use std::path::Path;

use assert_cmd::assert::Assert;
use assert_cmd::Command;
use tempfile::TempDir;

use crate::end_to_end;

const HEADER: &str = "import { PhylumApi } from 'phylum';";

/// Test Phylum API functions.
#[tokio::test]
pub async fn api() {
    let tempdir = TempDir::new().unwrap();
    let tempdir = tempdir.path();

    let lockfile = end_to_end::create_lockfile(tempdir);
    let config = end_to_end::create_config(&tempdir);
    let project = end_to_end::create_project().await;

    with_extension(&config, "console.log(await PhylumApi.getUserInfo())", |assert| {
        assert.success().stdout(predicates::str::contains("email"));
    });

    with_extension(&config, "console.log(await PhylumApi.getAccessToken())", |assert| {
        assert.success().stdout(predicates::str::contains("ey"));
    });

    with_extension(&config, "console.log(await PhylumApi.getRefreshToken())", |assert| {
        assert.success().stdout(predicates::str::contains("ey"));
    });

    with_extension(
        &config,
        "console.log(await PhylumApi.getPackageDetails('express', '4.18.1', 'npm'))",
        |assert| {
            assert.success().stdout(predicates::str::contains("vulnerability: 1"));
        },
    );

    let project_details = format!("console.log(await PhylumApi.getProjectDetails({project:?}))");
    with_extension(&config, &project_details, |assert| {
        assert.success().stdout(predicates::str::contains("name: \"integration-tests\""));
    });

    let parse_lockfile = format!(
        "const packages = await PhylumApi.parseLockfile({lockfile:?}, 'yarn');
        console.log(packages);",
    );
    with_extension(&config, &parse_lockfile, |assert| {
        assert.success().stdout("[ { name: \"accepts\", version: \"1.3.8\", type: \"npm\" } ]\n");
    });

    let analyze = format!(
        "const jobId = await PhylumApi.analyze({lockfile:?}, {project:?});
        console.log(await PhylumApi.getJobStatus(jobId));
    "
    );
    with_extension(&config, &analyze, |assert| {
        assert.success().stdout(predicates::str::contains("name: \"accepts\""));
    });
}

/// Ensure shared state is async and thread safe.
#[test]
pub fn async_state() {
    let tempdir = TempDir::new().unwrap();
    let tempdir = tempdir.path();

    let config = end_to_end::create_config(&tempdir);

    with_extension(
        &config,
        r#"
        const promises = [];
        promises.push(PhylumApi.getUserInfo());
        promises.push(PhylumApi.getUserInfo());
        promises.push(PhylumApi.getUserInfo());
        promises.push(PhylumApi.getUserInfo());
        promises.push(PhylumApi.getUserInfo());
        await Promise.all(promises);
    "#,
        |assert| {
            assert.success();
        },
    );
}

fn with_extension<F>(config: &Path, code: &str, mut f: F)
where
    F: FnMut(Assert),
{
    let tempdir = TempDir::new().unwrap();
    let tempdir = tempdir.path();

    // Create skeleton extension.
    Command::cargo_bin("phylum")
        .unwrap()
        .current_dir(tempdir)
        .args(&["extension", "new", "test-ext"])
        .assert()
        .success();

    // Overwrite skeleton code.
    let main = tempdir.join("test-ext").join("main.ts");
    fs::write(main, format!("{HEADER}\n{code}").as_bytes()).unwrap();

    // Install extension.
    Command::cargo_bin("phylum")
        .unwrap()
        .current_dir(tempdir)
        .env("XDG_DATA_HOME", tempdir)
        .args(&["extension", "install", "./test-ext"])
        .assert()
        .success();

    // Execute extension.
    let config_path = config.to_string_lossy();
    let assert = Command::cargo_bin("phylum")
        .unwrap()
        .env("XDG_DATA_HOME", tempdir)
        .args(&["--config", &config_path, "test-ext"])
        .assert();

    // Run extension assertions.
    f(assert);
}
