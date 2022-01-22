use assert_cmd::Command;

use phylum_cli::types::JobDescriptor;

fn is_sub<T: PartialEq>(haystack: &[T], needle: &[T]) -> bool {
    haystack.windows(needle.len()).any(|c| c == needle)
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn ping_system() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.arg("ping").assert();
    assert.success().stdout("\"Alive\"\n");
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn get_basic_status() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.arg("status").assert();

    let output = &assert.get_output().stderr;
    assert!(is_sub(output, b"success"));
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn get_job_status() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.arg("status").assert();

    let resp = String::from_utf8_lossy(&assert.get_output().stdout);
    let obj: Vec<JobDescriptor> = serde_json::from_str(&resp).unwrap();

    let job_id = obj[0].job_id;

    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.args(&["status", "-i", &job_id.to_string()]).assert();

    let output = &assert.get_output().stderr;
    assert!(is_sub(output, b"success"));
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn get_job_status_non_existent_job() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd
        .args(&["status", "-i", "ffffffff-ffff-ffff-ffff-ffffffffffff"])
        .assert();

    let output = assert.get_output();
    assert_eq!(output.stdout, b"");
    assert!(is_sub(&output.stderr, b"404 Not Found"));
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn get_package_status() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.arg("status").assert();

    let resp = String::from_utf8_lossy(&assert.get_output().stdout);
    let jobs: Vec<JobDescriptor> = serde_json::from_str(&resp).unwrap();

    let name = jobs[0].packages[0].name.to_string();
    let version = jobs[0].packages[0].version.to_string();

    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.args(&["status", "-n", &name, "-v", &version]).assert();

    let output = &assert.get_output().stderr;
    assert!(is_sub(output, b"success"));
}

#[test]
#[cfg_attr(not(feature = "phylum-online"), ignore)]
fn get_package_status_detailed() {
    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd.arg("status").assert();

    let resp = String::from_utf8_lossy(&assert.get_output().stdout);
    let jobs: Vec<JobDescriptor> = serde_json::from_str(&resp).unwrap();

    let name = jobs[0].packages[0].name.to_string();
    let version = jobs[0].packages[0].version.to_string();

    let mut cmd = Command::cargo_bin("phylum").unwrap();
    let assert = cmd
        .args(&["status", "-n", &name, "-v", &version])
        .arg("-V")
        .assert();

    let output = &assert.get_output().stderr;
    assert!(is_sub(output, b"success"));
}
