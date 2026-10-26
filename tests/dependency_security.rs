#[test]
fn locked_update_and_server_dependencies_include_security_fixes() {
    let lock: toml::Value = toml::from_str(include_str!("../Cargo.lock")).unwrap();
    let packages = lock["package"].as_array().unwrap();
    for (name, minimum, required) in [
        ("tar", "0.4.46", true),
        ("rustls", "0.23.46", true),
        ("zip", "9.0.0", true),
        ("time", "0.3.47", false),
        ("quick-xml", "0.41.0", false),
        ("anyhow", "1.0.103", true),
        ("bytes", "1.11.1", true),
        ("tokio", "1.53.3", true),
    ] {
        let versions: Vec<_> = packages
            .iter()
            .filter(|package| package["name"].as_str() == Some(name))
            .map(|package| semver::Version::parse(package["version"].as_str().unwrap()).unwrap())
            .collect();
        assert!(
            !required || !versions.is_empty(),
            "Missing dependency: {name}"
        );
        let minimum = semver::Version::parse(minimum).unwrap();
        for version in versions {
            assert!(
                version >= minimum,
                "{name} {version} predates security fixes ({minimum})"
            );
        }
    }
}

#[test]
fn ci_runs_dependency_audit_as_a_required_job() {
    #[derive(serde::Deserialize)]
    struct Workflow {
        jobs: std::collections::HashMap<String, Job>,
    }
    #[derive(serde::Deserialize)]
    struct Job {
        steps: Vec<Step>,
        #[serde(default, rename = "continue-on-error")]
        continue_on_error: bool,
    }
    #[derive(serde::Deserialize)]
    struct Step {
        run: Option<String>,
        #[serde(default, rename = "continue-on-error")]
        continue_on_error: bool,
    }
    let workflow: Workflow =
        serde_saphyr::from_str(include_str!("../.github/workflows/ci.yaml")).unwrap();
    let job = &workflow.jobs["audit"];
    assert!(!job.continue_on_error);
    let step = job
        .steps
        .iter()
        .find(|step| step.run.as_deref() == Some("cargo audit"))
        .unwrap();
    assert!(!step.continue_on_error);
}

#[test]
fn frontmatter_parser_has_no_unmaintained_libyaml_dependency() {
    let lock: toml::Value = toml::from_str(include_str!("../Cargo.lock")).unwrap();
    for package in lock["package"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        assert!(
            !matches!(name, "serde_yaml" | "unsafe-libyaml"),
            "Unmaintained YAML dependency: {name}"
        );
    }
}
