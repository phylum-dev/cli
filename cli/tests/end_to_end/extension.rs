use phylum_cli::commands::extensions::permissions::{Permission, Permissions};

use crate::common::{create_lockfile, create_project, TestCli};

/// Test Phylum API functions.
#[tokio::test]
pub async fn get_user_info() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getUserInfo())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("email"));
}

#[tokio::test]
pub async fn get_access_token() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getAccessToken())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("ey"));
}

#[tokio::test]
pub async fn get_refresh_token() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getRefreshToken())")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("ey"));
}

#[tokio::test]
pub async fn get_package_details() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getPackageDetails('express', '4.18.1', 'npm'))")
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("vulnerability: 1"));
}

#[test]
pub fn get_current_project() {
    let test_cli = TestCli::builder().cwd_temp().with_config(None).build();

    test_cli
        .extension("console.log(Phylum.getCurrentProject())")
        .build()
        .run()
        .success()
        .stdout("null\n");
}

#[tokio::test]
pub async fn get_groups() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getGroups())")
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
                await Phylum.deleteProject("create_and_delete")
            } catch (e) {
            }

            let newPrj = await Phylum.createProject("create_and_delete")
            let existingPrj = await Phylum.createProject("create_and_delete")

            if (newPrj.id !== existingPrj.id) {
                throw `ERROR IDs: ${newPrj.id} vs ${existingPrj.id}`
            }

            if (newPrj.status != "Created") {
                throw `ERROR newPrj.status = ${newPrj.status}`
            }

            if (existingPrj.status != "Exists") {
                throw `ERROR existingPrj.status = ${existingPrj.status}`
            }
        "#,
        )
        .build()
        .run()
        .success();
}

#[tokio::test]
pub async fn get_projects() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension("console.log(await Phylum.getProjects())")
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
        "const lockfile = await Phylum.parseDependencyFile({lockfile:?}, 'yarn');
         console.log(JSON.stringify(lockfile));",
    );
    test_cli
        .extension(&parse_lockfile)
        .with_permissions(permissions)
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains(
            ",\"packages\":[{\"name\":\"accepts\",\"version\":\"1.3.8\",\"type\":\"npm\"}],\"\
             format\":\"yarn\"}",
        ));
}

#[tokio::test]
pub async fn get_job_status() {
    let test_cli = TestCli::builder().with_config(None).build();

    let project = create_project().await;
    let analyze = format!(
        "
        const pkg = {{ name: 'typescript', version: '4.7.4', type: 'npm', lockfile: \
         'package-lock.json' }};
        const jobId = await Phylum.analyze([pkg], {project:?});
        console.log(await Phylum.getJobStatus(jobId));"
    );

    test_cli
        .extension(&analyze)
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("is_failure: "));
}

#[tokio::test]
pub async fn check_packages() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            "
            const pkg = { name: 'typescript', version: '4.7.4', type: 'npm' };
            const res = await Phylum.checkPackages([pkg]);
            console.log(res);",
        )
        .build()
        .run()
        .success()
        .stdout(predicates::str::contains("is_failure: "));
}

/// Ensure shared state is async and thread safe.
#[test]
pub fn async_state() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            r#"
            const promises = [];
            promises.push(Phylum.getUserInfo());
            promises.push(Phylum.getUserInfo());
            promises.push(Phylum.getUserInfo());
            promises.push(Phylum.getUserInfo());
            promises.push(Phylum.getUserInfo());
            await Promise.all(promises);
        "#,
        )
        .build()
        .run()
        .success();
}

#[tokio::test]
pub async fn rest_api() {
    let test_cli = TestCli::builder().with_config(None).build();

    test_cli
        .extension(
            "
            const reply = await Phylum.fetch(Phylum.ApiVersion.V0, '/health');
            console.log(await reply.json());
        ",
        )
        .with_permissions(Permissions { net: Permission::Boolean(true), ..Permissions::default() })
        .build()
        .run()
        .success()
        .stdout("{ response: \"alive!\" }\n");
}
