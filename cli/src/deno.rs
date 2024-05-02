//! Deno runtime for extensions.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;

use anyhow::{anyhow, Context, Error, Result};
use console::style;
use dashmap::DashMap;
use deno_ast::{EmitOptions, MediaType, ParseParams, SourceTextInfo, TranspiledSource};
use deno_runtime::deno_core::error::JsError;
use deno_runtime::deno_core::{
    Extension, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType,
    ResolutionKind, SourceMapGetter,
};
use deno_runtime::permissions::{Permissions, PermissionsContainer, PermissionsOptions};
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::{fmt_errors, BootstrapOptions};
use futures::future::BoxFuture;
use tokio::fs;
use url::Url;

use crate::api::PhylumApi;
use crate::commands::extensions::state::ExtensionState;
use crate::commands::extensions::{api, extension};
use crate::commands::{CommandResult, ExitCode};

/// Load Phylum API for module injection.
const EXTENSION_API: &str = include_str!("../../extensions/phylum.ts");

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
        ops: api::api_decls().into(),
        op_state_fn: Some(Box::new(|deno_state| deno_state.put(state))),
        ..Default::default()
    };

    let main_module =
        deno_core::resolve_path(&extension.entry_point().to_string_lossy(), &PathBuf::from("."))?;

    let bootstrap =
        BootstrapOptions { args, user_agent: "phylum-cli/extension".into(), ..Default::default() };

    let module_loader = Rc::new(ExtensionsModuleLoader::new(extension.path()));
    let source_map_getter: Box<dyn SourceMapGetter> = Box::new(module_loader.clone());

    let origin_storage_dir = extension.state_path();

    let options = WorkerOptions {
        origin_storage_dir,
        module_loader,
        bootstrap,
        source_map_getter: Some(source_map_getter),
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
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        let extension_path = self.extension_path.clone();
        let source_mapper = self.source_mapper.clone();

        Box::pin(async move {
            // Inject Phylum API module.
            if module_specifier.as_str() == "deno:phylum" {
                return phylum_module(&source_mapper);
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
            let mut code = match module_specifier.scheme() {
                "file" => {
                    ExtensionsModuleLoader::load_from_filesystem(&extension_path, &module_specifier)
                        .await?
                },
                "https" => ExtensionsModuleLoader::load_from_remote(&module_specifier).await?,
                _ => return Err(anyhow!("Unsupported module specifier: {}", module_specifier)),
            };

            if should_transpile {
                let transpiled =
                    source_mapper.transpile(module_specifier.to_string(), &code, media_type)?;
                code.clone_from(&transpiled.text);
            } else {
                source_mapper.source_cache.insert(module_specifier.to_string(), code.clone());
            }

            Ok(ModuleSource::new(module_type, code.into(), &module_specifier))
        })
    }
}

impl SourceMapGetter for ExtensionsModuleLoader {
    fn get_source_map(&self, file_name: &str) -> Option<Vec<u8>> {
        let transpiled = self.source_mapper.transpiled_cache.get(file_name)?;
        transpiled.source_map.clone().map(|map| map.into_bytes())
    }

    fn get_source_line(&self, file_name: &str, line_number: usize) -> Option<String> {
        let source = self.source_mapper.source_cache.get(file_name)?;
        source.lines().nth(line_number).map(|line| line.to_owned())
    }
}

/// Module source map cache.
#[derive(Default)]
struct SourceMapper {
    transpiled_cache: DashMap<String, TranspiledSource>,
    source_cache: DashMap<String, String>,
}

impl SourceMapper {
    fn new() -> Self {
        Self::default()
    }

    /// Transpile code to JavaScript.
    fn transpile(
        &self,
        specifier: impl Into<String>,
        code: impl Into<String>,
        media_type: MediaType,
    ) -> Result<TranspiledSource> {
        let specifier = specifier.into();

        // Load module if it is not in the cache.
        if !self.transpiled_cache.contains_key(&specifier) {
            let code = code.into();

            // Add the original code to the cache.
            self.source_cache.insert(specifier.clone(), code.clone());

            // Parse module.
            let parsed = deno_ast::parse_module(ParseParams {
                text_info: SourceTextInfo::from_string(code),
                specifier: specifier.clone(),
                capture_tokens: false,
                scope_analysis: false,
                maybe_syntax: None,
                media_type,
            })?;

            // Transpile to JavaScript.
            let options = EmitOptions { inline_source_map: false, ..EmitOptions::default() };
            let transpiled = parsed.transpile(&options)?;

            // Insert into our cache.
            self.transpiled_cache.insert(specifier.clone(), transpiled);
        }

        // Clone fields manually, since derive is missing.
        let transpiled = self.transpiled_cache.get(&specifier).unwrap();
        Ok(TranspiledSource {
            source_map: transpiled.source_map.clone(),
            text: transpiled.text.clone(),
        })
    }
}

/// Load the internal Phylum API module
fn phylum_module(mapper: &SourceMapper) -> Result<ModuleSource> {
    let module_url = ModuleSpecifier::parse("deno:phylum").unwrap();
    let transpiled = mapper.transpile(module_url.as_str(), EXTENSION_API, MediaType::TypeScript)?;
    let code = transpiled.text.clone();

    Ok(ModuleSource::new(ModuleType::JavaScript, code.into(), &module_url))
}
