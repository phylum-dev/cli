use std::str::FromStr;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
//use pyo3::types::PyDict;
//use pyo3::wrap_pyfunction;

use cli::api::PhylumApi as RustPhylumApi;
use cli::types::Token as RustToken;
use cli::types::{JobId, PackageDescriptor, PackageType};
use cli::Error;

#[pyclass]
struct Token {
    #[pyo3(get)]
    access: String,
    #[pyo3(get)]
    refresh: String,
}

#[pyclass]
#[text_signature = "(base_url)"]
struct PhylumApi {
    api: RustPhylumApi,
}

#[pymethods]
impl PhylumApi {
    #[new]
    pub fn new(base_url: &str) -> PyResult<Self> {
        RustPhylumApi::new(base_url)
            .map(|api| PhylumApi { api })
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to create new api instance: {:?}", e))
            })
    }

    /// Authenticate to the system
    ///
    ///   login
    ///     account username
    ///   pass
    ///     account password
    ///
    /// Returns a `Token` object consisting of both and access and refresh token
    #[text_signature = "(login, pass)"]
    pub fn authenticate(&mut self, login: &str, pass: &str) -> PyResult<Token> {
        self.api
            .authenticate(login, pass)
            .map(|t: RustToken| Token {
                access: t.access_token,
                refresh: t.refresh_token,
            })
            .map_err(|e: Error| PyRuntimeError::new_err(format!("Failed to authenticate: {:?}", e)))
    }

    /// Submit a package request to the system
    ///
    ///   name
    ///     The package name (e.g. `react`)
    ///   version
    ///     The package version (e.g. `16.13.1`)
    ///   type
    ///     The package type (currently only supports `npm`)
    ///
    /// Returns a job id
    #[text_signature = "(name, version, type=\"npm\")"]
    #[args(name, version, r#type = "\"npm\"")]
    pub fn submit_request(&mut self, name: &str, version: &str, r#type: &str) -> PyResult<String> {
        let pkg_type = PackageType::from_str(r#type).unwrap_or(PackageType::Npm);
        let pkg = PackageDescriptor {
            name: name.to_string(),
            version: version.to_string(),
            r#type: pkg_type,
        };
        self.api
            .submit_request(&[pkg])
            .map(|j: JobId| j.to_string())
            .map_err(|e: Error| {
                PyRuntimeError::new_err(format!("Failed to submit package request: {:?}", e))
            })
    }

    /// Get the status of a previously submitted job
    ///
    ///   job_id
    ///     The uuid returned by a call to `submit_request`
    ///
    /// Returns a dictionary containing status information for the request
    #[text_signature = "(job_id)"]
    pub fn get_status(&mut self, job_id: &str) -> PyResult<String> {
        let j = JobId::from_str(job_id)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid job id: {:?}", e)))?;
        let resp = self.api.get_status(&j).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to get package status: {:?}", e))
        })?;

        // TODO: we really should return this as a Python dict, not a json string
        let json = serde_json::to_string(&resp).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to serialize response: {:?}", e))
        })?;

        /*
        let gil = Python::acquire_gil();
        let py = gil.python();
        let locals = PyDict::new(py);
        locals.set_item("data", json);
        py.run("import json; result = json.loads(data)", None, Some(locals)).unwrap();
        let res = locals.get_item("result").unwrap().extract::<&PyDict>().unwrap();
        */

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
}

#[pymodule]
fn cli_python(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PhylumApi>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
