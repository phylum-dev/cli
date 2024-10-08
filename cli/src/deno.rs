//! Deno runtime for extensions.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{anyhow, Context, Error, Result};
use console::style;
use dashmap::DashMap;
use deno_ast::{
    EmitOptions, EmittedSourceBytes, MediaType, ParseParams, SourceMapOption, TranspileOptions,
};
use deno_core::error::JsError;
use deno_core::{
    include_js_files, Extension, ExtensionFileSource, ModuleCodeString, ModuleLoadResponse,
    ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType, RequestedModuleType,
    ResolutionKind,
};
use deno_runtime::deno_permissions::{Permissions, PermissionsContainer, PermissionsOptions};
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::{fmt_errors, BootstrapOptions};
use futures::future::BoxFuture;
use tokio::fs;
use url::Url;

use crate::api::PhylumApi;
use crate::commands::extensions::state::ExtensionState;
use crate::commands::extensions::{api, extension};
use crate::commands::{CommandResult, ExitCode};

/// Internal extension module that creates global Phylum object.
const EXTENSION_API: &[ExtensionFileSource] = &include_js_files!(
    phylum_api
    dir "js",
    "api.js",
    "api_version.js",
    "main.js");

/// Importable "phylum" module for backwards compatibility.
const PHYLUM_MODULE: &str = "
export const PhylumApi = Phylum;
export const ApiVersion = Phylum.ApiVersion;
";

/// Execute Phylum extension.
pub async fn run(
    api: BoxFuture<'static, Result<PhylumApi>>,
    extension: extension::Extension,
    args: Vec<String>,
) -> CommandResult {
    let state = ExtensionState::new(api, extension.clone());
    let phylum_api = Extension {
        name: "phylum-ext",
        middleware_fn: Some(Box::new(|op| match op.name {
            "op_request_permission" => op.disable(),
            _ => op,
        })),
        esm_files: Cow::Borrowed(EXTENSION_API),
        esm_entry_point: Some("ext:phylum_api/main.js"),
        ops: api::api_decls().into(),
        op_state_fn: Some(Box::new(|deno_state| deno_state.put(state))),
        ..Default::default()
    };

    let main_module = deno_core::resolve_path(extension.entry_point(), &PathBuf::from("."))?;

    let bootstrap =
        BootstrapOptions { args, user_agent: "phylum-cli/extension".into(), ..Default::default() };

    let module_loader = Rc::new(ExtensionsModuleLoader::new(extension.path()));

    let origin_storage_dir = extension.state_path();

    let options = WorkerOptions {
        origin_storage_dir,
        module_loader,
        bootstrap,
        extensions: vec![phylum_api],
        ..Default::default()
    };

    // Build permissions object from extension's requested permissions.
    let permissions_options = PermissionsOptions::from(&*extension.permissions());
    let worker_permissions = Permissions::from_options(&permissions_options)?;
    let permissions_container = PermissionsContainer::new(worker_permissions);

    // Initialize Deno runtime.
    let mut worker =
        MainWorker::bootstrap_from_options(main_module.clone(), permissions_container, options);

    // Execute extension code.
    if let Err(error) = worker.execute_main_module(&main_module).await {
        return print_js_error(error);
    }
    if let Err(error) = worker.run_event_loop(false).await {
        return print_js_error(error);
    }

    Ok(ExitCode::Ok)
}

/// Pretty-print an anyhow error as Deno JS error.
fn print_js_error(error: Error) -> CommandResult {
    let js_error: JsError = error.downcast::<JsError>()?;

    // Remove flag from permission errors.
    if let Some((message, _)) = js_error
        .message
        .as_ref()
        .and_then(|message| message.split_once(", run again with the --allow"))
    {
        return Err(anyhow!(message.to_owned()));
    }

    eprintln!("{}: {}", style("Extension error").red(), fmt_errors::format_js_error(&js_error));

    Ok(ExitCode::JsError)
}

/// See https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs.
struct ExtensionsModuleLoader {
    extension_path: Rc<PathBuf>,
    source_mapper: Rc<SourceMapper>,
}

impl ExtensionsModuleLoader {
    fn new(extension_path: PathBuf) -> Self {
        Self {
            extension_path: Rc::new(extension_path),
            source_mapper: Rc::new(SourceMapper::new()),
        }
    }

    async fn load_from_filesystem(extension_path: &Path, path: &Url) -> Result<String> {
        let path = path.to_file_path().map_err(|_| anyhow!("{path:?}: is not a path"))?;

        let extension_path = fs::canonicalize(&extension_path)
            .await
            .with_context(|| anyhow!("Invalid extension directory: {extension_path:?}"))?;
        let path = fs::canonicalize(&path)
            .await
            .with_context(|| anyhow!("Invalid extension module: {path:?}"))?;

        if !path.starts_with(extension_path) {
            return Err(anyhow!(
                "`{}`: importing from paths outside of the extension's directory is not allowed",
                path.to_string_lossy(),
            ));
        }

        Ok(fs::read_to_string(path).await?)
    }

    async fn load_from_remote(path: &Url) -> Result<String> {
        let response = reqwest::get(path.clone()).await?;
        Ok(response.text().await?)
    }
}

impl ModuleLoader for ExtensionsModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier> {
        if specifier == "phylum" {
            Ok(ModuleSpecifier::parse("deno:phylum")?)
        } else {
            Ok(deno_core::resolve_import(specifier, referrer)?)
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let module_specifier = module_specifier.clone();
        let extension_path = self.extension_path.clone();
        let source_mapper = self.source_mapper.clone();

        ModuleLoadResponse::Async(Box::pin(async move {
            // Inject Phylum API module.
            if module_specifier.as_str() == "deno:phylum" {
                return phylum_module();
            }

            // Determine source file type.
            // We do not care about invalid URLs yet: This match statement is inexpensive,
            // bears no risk and does not do I/O -- it operates fully off of the
            // contents of the URL.
            let media_type = MediaType::from_specifier(&module_specifier);
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                },
                MediaType::TypeScript
                | MediaType::Jsx
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => return Err(anyhow!("Unknown JS module format: {}", module_specifier)),
            };

            // Load either a local file under the extensions directory, or a Deno standard
            // library module. Reject all URLs that do not fit these two use
            // cases.
            let code = match module_specifier.scheme() {
                "file" => {
                    ExtensionsModuleLoader::load_from_filesystem(&extension_path, &module_specifier)
                        .await?
                },
                "https" => ExtensionsModuleLoader::load_from_remote(&module_specifier).await?,
                _ => return Err(anyhow!("Unsupported module specifier: {}", module_specifier)),
            };

            let module_source = if should_transpile {
                let transpiled = source_mapper.transpile(
                    module_specifier.to_string(),
                    code.into(),
                    media_type,
                )?;
                ModuleSourceCode::Bytes(transpiled.source.into_boxed_slice().into())
            } else {
                source_mapper
                    .source_cache
                    .insert(module_specifier.to_string(), code.clone().into());
                ModuleSourceCode::String(code.into())
            };

            Ok(ModuleSource::new(module_type, module_source, &module_specifier, None))
        }))
    }

    fn get_source_map(&self, file_name: &str) -> Option<Vec<u8>> {
        let transpiled = self.source_mapper.transpiled_cache.get(file_name)?;
        transpiled.source_map.clone()
    }

    fn get_source_mapped_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
        let source = self.source_mapper.source_cache.get(file_name)?;
        source.lines().nth(line_number).map(|line| line.to_owned())
    }
}

/// Module source map cache.
#[derive(Default)]
struct SourceMapper {
    transpiled_cache: DashMap<String, EmittedSourceBytes>,
    source_cache: DashMap<String, Arc<str>>,
}

impl SourceMapper {
    fn new() -> Self {
        Self::default()
    }

    /// Transpile code to JavaScript.
    fn transpile(
        &self,
        specifier: impl Into<String>,
        code: Arc<str>,
        media_type: MediaType,
    ) -> Result<EmittedSourceBytes> {
        let specifier = specifier.into();

        // Load module if it is not in the cache.
        if !self.transpiled_cache.contains_key(&specifier) {
            // Add the original code to the cache.
            self.source_cache.insert(specifier.clone(), code.clone());

            // Parse module.
            let parsed = deno_ast::parse_module(ParseParams {
                text: code,
                specifier: specifier.parse()?,
                capture_tokens: false,
                scope_analysis: false,
                maybe_syntax: None,
                media_type,
            })?;

            // Transpile to JavaScript.
            let emit_options =
                EmitOptions { source_map: SourceMapOption::Separate, ..EmitOptions::default() };
            let transpile_options = TranspileOptions::default();
            let transpiled = parsed.transpile(&transpile_options, &emit_options)?.into_source();

            // Insert into our cache.
            self.transpiled_cache.insert(specifier.clone(), transpiled);
        }

        let transpiled = self.transpiled_cache.get(&specifier).unwrap();
        Ok(transpiled.clone())
    }
}

/// Load the internal Phylum API module
fn phylum_module() -> Result<ModuleSource> {
    let module_url = ModuleSpecifier::parse("deno:phylum").unwrap();

    Ok(ModuleSource::new(
        ModuleType::JavaScript,
        ModuleSourceCode::String(ModuleCodeString::from_static(PHYLUM_MODULE)),
        &module_url,
        None,
    ))
}
