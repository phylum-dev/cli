use phylum_cli::commands::extensions::permissions::{Permission, Permissions};

use crate::common::{create_lockfile, create_project, TestCli};

/// Test Phylum API functions.
#[tokio::test]
pub async fn get_user_info() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getUserInfo())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("email"));
}

#[tokio::test]
pub async fn get_access_token() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getAccessToken())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("ey"));
}

#[tokio::test]
pub async fn get_refresh_token() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getRefreshToken())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("ey"));
}

#[tokio::test]
pub async fn get_package_details() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getPackageDetails('express', '4.18.1', 'npm'))")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("vulnerability: 1"));
}

#[test]
pub fn get_current_project() {
    let test_cli = TestCli::builder().cwd_temp().with_config(None).build();

    test_cli
        .extension("console.log(PhylumApi.getCurrentProject())")
        .build()
        .run()
        .success()
        .stdout("null\n");
}

#[tokio::test]
pub async fn get_groups() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getGroups())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("groups"));
}

#[tokio::test]
pub async fn create_and_delete_project() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            r#"
            try {
                await PhylumApi.deleteProject("create_and_delete_test_project")
            } catch (e) {
            }

            let projectId = await PhylumApi.createProject("create_and_delete")
            let duplicatedProjectId = await PhylumApi.createProject("create_and_delete")
            if (projectId === duplicatedProjectId) {
                console.log("OK")
            } else {
                console.log("ERROR " + projectId + " " + duplicatedProjectId)
            }

        "#,
        )
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("["));
}

#[tokio::test]
pub async fn get_projects() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await PhylumApi.getProjects())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("["));
}

#[tokio::test]
pub async fn parse_lockfile() {
    let test_cli = TestCli::builder().with_config(None).build();

    let lockfile = create_lockfile(test_cli.temp_path());
    let lockfile_str = lockfile.to_string_lossy().into_owned();
    let permissions =
        Permissions { read: Permission::List(vec![lockfile_str]), ..Permissions::default() };

    let parse_lockfile = format!(
        "const lockfile = await PhylumApi.parseLockfile({lockfile:?}, 'yarn');
         console.log(lockfile);",
    );
    test_cli
        .extension(&parse_lockfile)
        .with_permissions(permissions)
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains(
            r#"{ packages: [ { name: "accepts", version: "1.3.8" } ], package_type: "npm" }"#,
        ));
}

#[tokio::test]
pub async fn get_job_status() {
    let test_cli = TestCli::builder().with_config(None).build();

    let project = create_project().await;
    let analyze = format!(
        "
        const pkg = {{ name: 'typescript', version: '4.7.4'}};
        const jobId = await PhylumApi.analyze('npm', [pkg], {project:?});
        console.log(await PhylumApi.getJobStatus(jobId));"
    );

    test_cli
        .extension(&analyze)
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains(r#"name: "typescript""#));
}

/// Ensure shared state is async and thread safe.
#[test]
pub fn async_state() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            r#"
            const promises = [];
            promises.push(PhylumApi.getUserInfo());
            promises.push(PhylumApi.getUserInfo());
            promises.push(PhylumApi.getUserInfo());
            promises.push(PhylumApi.getUserInfo());
            promises.push(PhylumApi.getUserInfo());
            await Promise.all(promises);
        "#,
        )
        .build()
        .run()
        .success();
}
