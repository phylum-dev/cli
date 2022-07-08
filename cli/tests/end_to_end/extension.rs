use crate::common::{create_lockfile, create_project, TestCli};

/// Test Phylum API functions.
#[tokio::test]
pub async fn api() {
    let test_cli = TestCli::builder().with_config(true).build();

    let lockfile = create_lockfile(test_cli.temp_path());
    let project = create_project().await;

    {
        test_cli
            .create_extension("console.log(await PhylumApi.getUserInfo())")
            .run()
            .success()
            .stdout(predicates::str::contains("email"));
    }

    {
        test_cli
            .create_extension("console.log(await PhylumApi.getAccessToken())")
            .run()
            .success()
            .stdout(predicates::str::contains("ey"));
    }

    {
        test_cli
            .create_extension("console.log(await PhylumApi.getRefreshToken())")
            .run()
            .success()
            .stdout(predicates::str::contains("ey"));
    }

    {
        test_cli
            .create_extension(
                "console.log(await PhylumApi.getPackageDetails('express', '4.18.1', 'npm'))",
            )
            .run()
            .success()
            .stdout(predicates::str::contains("vulnerability: 1"));
    }

    {
        let project_details =
            format!("console.log(await PhylumApi.getProjectDetails({project:?}))");
        test_cli
            .create_extension(&project_details)
            .run()
            .success()
            .stdout(predicates::str::contains(r#"name: "integration-tests""#));
    }

    {
        let parse_lockfile = format!(
            "const packages = await PhylumApi.parseLockfile({lockfile:?}, 'yarn');
             console.log(packages);",
        );
        test_cli.create_extension(&parse_lockfile).run().success().stdout(
            predicates::str::contains(r#"[ { name: "accepts", version: "1.3.8", type: "npm" } ]"#),
        );
    }

    {
        let analyze = format!(
            "const jobId = await PhylumApi.analyze({lockfile:?}, {project:?});
             console.log(await PhylumApi.getJobStatus(jobId));"
        );
        test_cli
            .create_extension(&analyze)
            .run()
            .success()
            .stdout(predicates::str::contains(r#"name: "accepts""#));
    }
}

/// Ensure shared state is async and thread safe.
#[test]
pub fn async_state() {
    let test_cli = TestCli::builder().with_config(true).build();

    test_cli
        .create_extension(
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
        .run()
        .success();
}
