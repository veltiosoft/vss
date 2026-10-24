#[test]
fn locked_tls_dependencies_include_certificate_distrust_fixes() {
    let lock: toml::Value = toml::from_str(include_str!("../Cargo.lock")).unwrap();
    let packages = lock["package"].as_array().unwrap();
    for (name, minimum) in [("webpki-roots", "1.0.10"), ("rustls-webpki", "0.103.16")] {
        let requirement = semver::VersionReq::parse(&format!(">={minimum}")).unwrap();
        let versions: Vec<_> = packages
            .iter()
            .filter(|package| package["name"].as_str() == Some(name))
            .map(|package| semver::Version::parse(package["version"].as_str().unwrap()).unwrap())
            .collect();
        assert!(!versions.is_empty(), "Missing TLS dependency: {name}");
        for version in versions {
            assert!(
                requirement.matches(&version),
                "{name} {version} predates the certificate distrust fix ({minimum})"
            );
        }
    }
}

#[test]
fn trust_store_excludes_distrusted_emsign_root() {
    let distrusted_name = b"emSign ECC Root CA - G3";
    assert!(!webpki_roots::TLS_SERVER_ROOTS.iter().any(|anchor| {
        anchor
            .subject
            .as_ref()
            .windows(distrusted_name.len())
            .any(|name| name == distrusted_name)
    }));
}
