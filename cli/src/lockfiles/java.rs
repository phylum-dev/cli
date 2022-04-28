use std::io;
use std::path::Path;

use phylum_types::ecosystems::maven::{Dependency, Plugin, Project};
use phylum_types::types::package::{PackageDescriptor, PackageType};

use super::parsers::gradle_dep;
use crate::lockfiles::{ParseResult, Parseable};

pub struct Pom(String);
pub struct GradleDeps(String);

impl Parseable for GradleDeps {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(GradleDeps(std::fs::read_to_string(filename)?))
    }

    /// Parses `gradle-dependencies.txt` files into a vec of packages
    fn parse(&self) -> ParseResult {
        let (_, entries) =
            gradle_dep::parse(&self.0).map_err(|_e| "Failed to parse requirements file")?;
        Ok(entries)
    }
}

impl Parseable for Pom {
    fn new(filename: &Path) -> Result<Self, io::Error>
    where
        Self: Sized,
    {
        Ok(Pom(std::fs::read_to_string(filename)?))
    }

    /// Parses maven effecti-pom files into a vec of packages
    fn parse(&self) -> ParseResult {
        // Get plugin dependencies
        fn get_plugin_deps(plugins: &[Plugin]) -> Vec<Dependency> {
            plugins
                .iter()
                .flat_map(|plugin| {
                    let mut deps = plugin.dependencies.clone().unwrap_or_default().dependencies;
                    deps.push(Dependency {
                        group_id: plugin.group_id.clone(),
                        artifact_id: plugin.artifact_id.clone(),
                        version: plugin.version.clone(),
                        dtype: None,
                        classifier: None,
                        scope: None,
                        system_path: None,
                        exclusions: None,
                        optional: None,
                    });
                    deps
                })
                .collect::<Vec<_>>()
        }

        // Get project reference
        let pom: Project = serde_xml_rs::from_str(&self.0)?;

        // Get project dependencies
        let mut dependencies = pom.dependencies.unwrap_or_default().dependencies;

        // Get the reporting dependencies
        let mut reporting_dependencies = get_plugin_deps(
            &pom.reporting
                .unwrap_or_default()
                .plugins
                .unwrap_or_default(),
        );

        // Combine plugins and plugin dependencies
        let mut build_plugins = get_plugin_deps(
            &pom.build
                .as_ref()
                .and_then(|b| b.plugins.clone())
                .unwrap_or_default(),
        );

        // Get build artifacts
        let build_ext = &pom
            .build
            .unwrap_or_default()
            .extensions
            .unwrap_or_default()
            .iter()
            .map(|ext| Dependency {
                group_id: ext.group_id.clone(),
                artifact_id: ext.artifact_id.clone(),
                version: ext.version.clone(),
                dtype: None,
                classifier: None,
                scope: None,
                system_path: None,
                exclusions: None,
                optional: None,
            })
            .collect::<Vec<_>>();

        let mut profile_dependencies = pom
            .profiles
            .unwrap_or_default()
            .profiles
            .into_iter()
            .flat_map(|p| {
                let mut p_deps = p.dependencies.unwrap_or_default().dependencies;
                let p_report_plugins =
                    get_plugin_deps(&p.reporting.unwrap_or_default().plugins.unwrap_or_default());
                p_deps.extend(p_report_plugins);
                p_deps
            })
            .collect::<Vec<_>>();

        dependencies.append(&mut reporting_dependencies);
        dependencies.append(&mut build_plugins);
        dependencies.extend(build_ext.to_owned());
        dependencies.append(&mut profile_dependencies);
        dependencies.dedup();
        dependencies
            .iter()
            .filter_map(|dep| {
                dep.version.as_ref().map(|s| {
                    Ok(PackageDescriptor {
                        name: format!(
                            "{}:{}",
                            &dep.group_id.clone().unwrap_or_default(),
                            &dep.artifact_id.clone().unwrap_or_default()
                        ),
                        version: s.into(),
                        package_type: PackageType::Maven,
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_gradledeps() {
        let parser = GradleDeps::new(Path::new("tests/fixtures/gradle-dependencies.txt")).unwrap();

        let pkgs = parser.parse().unwrap();
        assert_eq!(pkgs.len(), 43);
        assert_eq!(pkgs[0].name, "com.google.guava:guava");
        assert_eq!(pkgs[0].version, "23.3-jre");
        assert_eq!(pkgs[0].package_type, PackageType::Maven);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "commons-codec:commons-codec");
        assert_eq!(last.version, "1.9");
        assert_eq!(last.package_type, PackageType::Maven);
    }

    #[test]
    fn lock_parse_effective_pom() {
        let parser = Pom::new(Path::new("tests/fixtures/effective-pom.xml")).unwrap();

        let mut pkgs = parser.parse().unwrap();
        pkgs.sort_by(|a, b| a.version.cmp(&b.version));
        assert_eq!(pkgs.len(), 16);
        assert_eq!(pkgs[0].name, "com.bitalino:bitalino-java-sdk");
        assert_eq!(pkgs[0].version, "1.1.0");
        assert_eq!(pkgs[0].package_type, PackageType::Maven);

        assert_eq!(pkgs[1].name, "org.codehaus.mojo:exec-maven-plugin");
        assert_eq!(pkgs[1].version, "1.2.1");
        assert_eq!(pkgs[1].package_type, PackageType::Maven);

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "org.apache.maven.plugins:maven-site-plugin");
        assert_eq!(last.version, "3.3");
        assert_eq!(last.package_type, PackageType::Maven);
    }
}
