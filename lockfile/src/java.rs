use std::ffi::OsStr;
use std::path::Path;

use anyhow::{anyhow, Context};
use nom::error::convert_error;
use nom::Finish;
use phylum_types::ecosystems::maven::{Dependency, Plugin, Project};
use phylum_types::types::package::PackageType;
use serde::Deserialize;

use super::parsers::gradle_dep;
use crate::{Package, PackageVersion, Parse};

pub struct Pom;
pub struct GradleLock;

impl Parse for GradleLock {
    /// Parses `gradle.lockfile` files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        let (_, entries) = gradle_dep::parse(data)
            .finish()
            .map_err(|e| anyhow!(convert_error(data, e)))
            .context("Failed to parse requirements file")?;
        Ok(entries)
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("gradle.lockfile"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("build.gradle"))
    }
}

impl Parse for Pom {
    /// Parses maven effective-pom files into a vec of packages
    fn parse(&self, data: &str) -> anyhow::Result<Vec<Package>> {
        // Parse effective-pom.xml.
        let pom: EffectivePom = serde_xml_rs::from_str(data)?;

        // Retrieve all dependencies.
        match pom {
            EffectivePom::Project(project) => self.project_dependencies(*project),
            EffectivePom::Workspace(workspace) => {
                // Retrieve all dependencies.
                let mut packages = Vec::new();
                for project in workspace.projects {
                    packages.append(&mut self.project_dependencies(project)?);
                }

                // Deduplicate between projects.
                packages.sort_unstable();
                packages.dedup();

                Ok(packages)
            },
        }
    }

    fn is_path_lockfile(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("effective-pom.xml"))
    }

    fn is_path_manifest(&self, path: &Path) -> bool {
        path.file_name() == Some(OsStr::new("pom.xml"))
    }
}

impl Pom {
    /// Get dependencies of a single project.
    fn project_dependencies(&self, project: Project) -> anyhow::Result<Vec<Package>> {
        // Get project dependencies
        let mut dependencies = project.dependencies.unwrap_or_default().dependencies;

        // Get the reporting dependencies
        let mut reporting_dependencies = Self::get_plugin_deps(
            &project.reporting.unwrap_or_default().plugins.unwrap_or_default(),
        );

        // Combine plugins and plugin dependencies
        let mut build_plugins = Self::get_plugin_deps(
            &project.build.as_ref().and_then(|b| b.plugins.clone()).unwrap_or_default(),
        );

        // Get build artifacts
        let build_ext = &project
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

        let mut profile_dependencies = project
            .profiles
            .unwrap_or_default()
            .profiles
            .into_iter()
            .flat_map(|p| {
                let mut p_deps = p.dependencies.unwrap_or_default().dependencies;
                let p_report_plugins = Self::get_plugin_deps(
                    &p.reporting.unwrap_or_default().plugins.unwrap_or_default(),
                );
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
                    Ok(Package {
                        name: format!(
                            "{}:{}",
                            &dep.group_id.clone().unwrap_or_default(),
                            &dep.artifact_id.clone().unwrap_or_default()
                        ),
                        version: PackageVersion::FirstParty(s.into()),
                        package_type: PackageType::Maven,
                    })
                })
            })
            .collect()
    }

    /// Get plugin dependencies.
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
}

#[derive(Deserialize)]
enum EffectivePom {
    #[serde(rename = "project")]
    Project(Box<Project>),
    #[serde(rename = "projects")]
    Workspace(Workspace),
}

#[derive(Deserialize)]
struct Workspace {
    #[serde(rename = "project")]
    projects: Vec<Project>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_parse_gradle() {
        let pkgs = GradleLock.parse(include_str!("../../tests/fixtures/gradle.lockfile")).unwrap();

        assert_eq!(pkgs.len(), 6);

        assert_eq!(pkgs[0].name, "com.google.code.findbugs:jsr305");
        assert_eq!(pkgs[0].version, PackageVersion::FirstParty("1.3.9".into()));

        assert_eq!(pkgs[2].name, "com.google.guava:guava");
        assert_eq!(pkgs[2].version, PackageVersion::FirstParty("23.3-jre".into()));

        assert_eq!(pkgs[5].name, "org.springframework:spring-core");
        assert_eq!(pkgs[5].version, PackageVersion::FirstParty("5.2.15.RELEASE".into()));
    }

    #[test]
    fn lock_parse_effective_pom() {
        let mut pkgs = Pom.parse(include_str!("../../tests/fixtures/effective-pom.xml")).unwrap();

        pkgs.sort_by(|a, b| a.version.cmp(&b.version));
        assert_eq!(pkgs.len(), 16);
        assert_eq!(pkgs[0].name, "com.bitalino:bitalino-java-sdk");
        assert_eq!(pkgs[0].version, PackageVersion::FirstParty("1.1.0".into()));

        assert_eq!(pkgs[1].name, "org.codehaus.mojo:exec-maven-plugin");
        assert_eq!(pkgs[1].version, PackageVersion::FirstParty("1.2.1".into()));

        let last = pkgs.last().unwrap();
        assert_eq!(last.name, "org.apache.maven.plugins:maven-site-plugin");
        assert_eq!(last.version, PackageVersion::FirstParty("3.3".into()));
    }

    #[test]
    fn lock_parse_workspace_effective_pom() {
        let pkgs =
            Pom.parse(include_str!("../../tests/fixtures/workspace-effective-pom.xml")).unwrap();

        assert_eq!(pkgs.len(), 88);

        let additional_dependency = Package {
            name: "io.phylum:fake-dependency".into(),
            version: PackageVersion::FirstParty("1.2.3".into()),
            package_type: PackageType::Maven,
        };

        assert!(pkgs.contains(&additional_dependency));
    }
}
