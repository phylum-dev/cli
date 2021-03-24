use std::str::FromStr;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
//use pyo3::types::IntoPyDict;
//use pyo3::types::PyDict;
//use pyo3::wrap_pyfunction;

use phylum_cli::api::PhylumApi as RustPhylumApi;
use phylum_cli::types::ApiToken as RustApiToken;
use phylum_cli::types::JwtToken as RustJwtToken;
use phylum_cli::types::Key;
use phylum_cli::types::{JobId, PackageDescriptor, PackageType, ProjectId, UserId};
use phylum_cli::Error;

#[pyclass]
struct JwtToken {
    #[pyo3(get)]
    access: String,
    #[pyo3(get)]
    refresh: Option<String>,
}

#[pyclass]
struct ApiToken {
    #[pyo3(get)]
    active: bool,
    #[pyo3(get)]
    key: String,
    #[pyo3(get)]
    user_id: String,
}

/// Create a new instance of the Phylum API
/// 
///   base_url
///     The base url for the api to connect to.
#[pyclass]
#[text_signature = "(base_url)"]
struct PhylumApi {
    api: RustPhylumApi,
}

#[pymethods]
impl PhylumApi {
    #[new]
    #[args(base_url = "\"https://api.phylum.io\"")]
    pub fn new(base_url: &str) -> PyResult<Self> {
        RustPhylumApi::new(base_url)
            .map(|api| PhylumApi { api })
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to create new api instance: {:?}", e))
            })
    }

    /// Register a new user in the system
    ///
    ///   email
    ///     account username
    ///   password
    ///     account password
    ///   first_name
    ///     user first name
    ///   last_name
    ///     user last name
    ///
    /// Returns a user id
    #[text_signature = "(email, password, first_name, last_name)"]
    pub fn register(
        &mut self,
        email: &str,
        password: &str,
        first_name: &str,
        last_name: &str,
    ) -> PyResult<String> {
        self.api
            .register(email, password, first_name, last_name)
            .map(|u: UserId| u.to_string())
            .map_err(|e: Error| {
                PyRuntimeError::new_err(format!("Failed to register user: {:?}", e))
            })
    }

    /// Authenticate to the system
    ///
    ///   login
    ///     account username
    ///   pass
    ///     account password
    ///
    /// Returns a `JwtToken` object consisting of both and access and refresh token
    #[text_signature = "(login, pass)"]
    pub fn authenticate(&mut self, login: &str, pass: &str) -> PyResult<JwtToken> {
        self.api
            .authenticate(login, pass)
            .map(|t: RustJwtToken| JwtToken {
                access: t.access_token,
                refresh: t.refresh_token,
            })
            .map_err(|e: Error| PyRuntimeError::new_err(format!("Failed to authenticate: {:?}", e)))
    }

    /// Refresh an existing JWT token
    ///
    ///   token
    ///     JWT token
    ///
    /// Returns a `JwtToken` object consisting of both and access and refresh token
    #[text_signature = "(token)"]
    pub fn refresh(&mut self, token: &JwtToken) -> PyResult<JwtToken> {
        let rtoken = RustJwtToken {
            access_token: token.access.to_owned(),
            refresh_token: token.refresh.to_owned(),
        };
        self.api
            .refresh(&rtoken)
            .map(|t: RustJwtToken| JwtToken {
                access: t.access_token,
                refresh: t.refresh_token,
            })
            .map_err(|e: Error| {
                PyRuntimeError::new_err(format!("Failed to refresh token: {:?}", e))
            })
    }

    /// Create a long-lived API token
    ///
    /// Returns a `ApiToken` object consisting of a key and user id
    pub fn create_api_token(&mut self) -> PyResult<ApiToken> {
        self.api
            .create_api_token()
            .map(|t: RustApiToken| ApiToken {
                active: t.active,
                key: t.key.to_string(),
                user_id: t.user_id.to_string(),
            })
            .map_err(|e: Error| {
                PyRuntimeError::new_err(format!("Failed to create api token: {:?}", e))
            })
    }

    /// Delete (de-activate) an API token
    pub fn delete_api_token(&mut self, key: &str) -> PyResult<()> {
        let key = Key::from_str(key)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid api key: {:?}", e)))?;
        self.api.delete_api_token(&key).map_err(|e: Error| {
            PyRuntimeError::new_err(format!("Failed to create api token: {:?}", e))
        })
    }

    /// Get a list of API tokens
    pub fn get_api_tokens(&mut self) -> PyResult<Vec<ApiToken>> {
        let tokens = self.api.get_api_tokens().map_err(|e: Error| {
            PyRuntimeError::new_err(format!("Failed to create api token: {:?}", e))
        })?;

        Ok(tokens
            .iter()
            .map(|t: &RustApiToken| ApiToken {
                active: t.active,
                key: t.key.to_string(),
                user_id: t.user_id.to_string(),
            })
            .collect::<Vec<_>>())
    }

    /// Set the api token to use for making package requests
    ///
    ///   token
    ///     an `ApiToken` returned by `create_api_token`
    pub fn set_api_token(&mut self, token: &ApiToken) -> PyResult<()> {
        let key = Key::from_str(&token.key)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid api key: {:?}", e)))?;
        let user_id = UserId::from_str(&token.user_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid api key: {:?}", e)))?;
        let rtoken = RustApiToken {
            active: token.active,
            key,
            user_id,
        };
        self.api.set_api_token(&rtoken).map_err(|e: Error| {
            PyRuntimeError::new_err(format!("Failed to create api token: {:?}", e))
        })
    }

    /// Submit a package request to the system
    ///
    ///   project
    ///     The project id to associate with this request
    ///   name
    ///     The package name (e.g. `react`)
    ///   version
    ///     The package version (e.g. `16.13.1`)
    ///   type
    ///     The package type (currently only supports `npm`)
    ///   label
    ///     The label to associate with this request (default: None)
    ///
    /// Returns a job id
    #[text_signature = "(project, name, version, type=\"npm\", label=None)"]
    #[args(project, name, version, r#type = "\"npm\"", label = "None")]
    pub fn submit_request(&mut self, project: &str, name: &str, version: &str, r#type: &str, label: Option<String>) -> PyResult<String> {
        let pkg_type = PackageType::from_str(r#type).unwrap_or(PackageType::Npm);
        let pkg = PackageDescriptor {
            name: name.to_string(),
            version: version.to_string(),
            r#type: pkg_type.to_owned(),
        };
        let proj_id = ProjectId::from_str(project)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid project id: {:?}", e)))?;

        self.api
            .submit_request(&pkg_type, &[pkg], true, true, proj_id, label)
            .map(|j: JobId| j.to_string())
            .map_err(|e: Error| {
                PyRuntimeError::new_err(format!("Failed to submit package request: {:?}", e))
            })
    }

    /// Get the status of a previously submitted job(s)
    ///
    ///   job_id
    ///     The uuid returned by a call to `submit_request`
    ///
    /// Returns a dictionary containing status information for the request
    #[text_signature = "(job_id)"]
    pub fn get_job_status(&mut self, job_id: &str) -> PyResult<String> {
        let j = JobId::from_str(job_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid job id: {:?}", e)))?;

        let job = self.api.get_job_status(&j).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to get job status: {:?}", e))
        })?;

        // TODO: we should return this as a Python dict, not a json string
        let json = serde_json::to_string(&job).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to serialize response: {:?}", e))
        })?;

        Ok(json)
    }

    /// Get the overall status for current jobs
    ///
    /// Returns a dictionary containing status information for the request
    pub fn get_status(&mut self) -> PyResult<String> {
        let jobs = self.api.get_status().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to get job status: {:?}", e))
        })?;

        // TODO: we should return this as a Python dict, not a json string
        let json = serde_json::to_string(&jobs).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to serialize response: {:?}", e))
        })?;

        Ok(json)
    }
 
    /// Cancel a job currently in progress
    ///
    ///   job_id
    ///     The uuid returned by a call to `submit_request`
    ///
    #[text_signature = "(job_id)"]
    pub fn cancel(&mut self, job_id: &str) -> PyResult<String> {
        let j = JobId::from_str(job_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid job id: {:?}", e)))?;
        let resp = self.api.cancel(&j).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to cancel request `{}`: {}", j, e))
        })?;
        Ok(resp.msg)
    }

    /// List the available heuristics
    /// 
    pub fn heuristics(&mut self) -> PyResult<Vec<String>> {
        let resp = self.api.query_heuristics().map_err(|e| {
            PyRuntimeError::new_err(format!("Could not query available heuristics: {}", e))
        })?;
        Ok(resp)
    }

    /// Submit a package to have heuristics run against it
    /// 
    ///   pkg_name
    ///     The name of the package to run heuristics against    
    ///   pkg_version
    ///     The version of the package to run heuristics against    
    ///   pkg_type
    ///     The type of the package to run heuristics against (default: "npm")
    ///   heuristics
    ///     A list of heuristics to run (if not provided, all available will be run)
    ///   include_deps
    ///     Heuristics should also be run against the packages dependencies (default: False)
    /// 
    #[text_signature = "(pkg_type=\"npm\", heuristics=None, include_deps=False)"]
    #[args(pkg_type = "\"npm\"", heuristics = "None", include_deps = false)]
    pub fn run_heuristics(&mut self, pkg_name: &str, pkg_version: &str, pkg_type: &str, heuristics: Option<Vec<String>>, include_deps: bool) -> PyResult<()> {
        let pkg = PackageDescriptor {
            name: pkg_name.to_string(),
            version: pkg_version.to_string(),
            r#type: PackageType::from_str(pkg_type).unwrap_or(PackageType::Npm),
        };
        let heuristics = heuristics.unwrap_or_default();
        let _resp = self.api.submit_heuristics(&pkg, &heuristics, include_deps).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to submit package `{}/{}` for heuristics: {}", pkg_name, pkg_version, e))
        })?;
        Ok(())
    }
}

#[pymodule]
fn cli_python(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PhylumApi>()?;
    Ok(())
}
