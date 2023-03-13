//! Deno runtime for extensions.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, Context, Error, Result};
use console::style;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_runtime::deno_core::error::JsError;
use deno_runtime::deno_core::{
    self, Extension, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType,
};
use deno_runtime::permissions::{Permissions, PermissionsOptions};
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::{colors, fmt_errors, BootstrapOptions};
use futures::future::BoxFuture;
use tokio::fs;
use url::Url;

use crate::api::PhylumApi;
use crate::commands::extensions::state::ExtensionState;
use crate::commands::extensions::{api, extension};
use crate::commands::{CommandResult, ExitCode};

/// Execute Phylum extension.
pub async fn run(
    api: BoxFuture<'static, Result<PhylumApi>>,
    extension: extension::Extension,
    args: Vec<String>,
) -> CommandResult {
    let phylum_api = Extension::builder()
        .middleware(|op| match op.name {
            "op_request_permission" => op.disable(),
            _ => op,
        })
        .ops(api::api_decls())
        .build();

    let main_module = deno_core::resolve_path(&extension.entry_point().to_string_lossy())?;

    let cpu_count = thread::available_parallelism().map(|p| p.get()).unwrap_or(1);

    let bootstrap = BootstrapOptions {
        cpu_count,
        args,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        user_agent: "phylum-cli/extension".into(),
        no_color: !colors::use_color(),
        is_tty: colors::is_tty(),
        enable_testing_features: Default::default(),
        debug_flag: Default::default(),
        ts_version: Default::default(),
        location: Default::default(),
        unstable: Default::default(),
        inspect: Default::default(),
    };

    let options = WorkerOptions {
        bootstrap,
        web_worker_pre_execute_module_cb: Arc::new(|_| {
            unimplemented!("web workers are not supported")
        }),
        web_worker_preload_module_cb: Arc::new(|_| unimplemented!("web workers are not supported")),
        create_web_worker_cb: Arc::new(|_| unimplemented!("web workers are not supported")),
        module_loader: Rc::new(ExtensionsModuleLoader::new(extension.path())),
        extensions: vec![phylum_api],
        seed: None,
        unsafely_ignore_certificate_errors: Default::default(),
        should_break_on_first_statement: Default::default(),
        compiled_wasm_module_store: Default::default(),
        shared_array_buffer_store: Default::default(),
        maybe_inspector_server: Default::default(),
        format_js_error_fn: Default::default(),
        get_error_class_fn: Default::default(),
        origin_storage_dir: Default::default(),
        broadcast_channel: Default::default(),
        cache_storage_dir: Default::default(),
        source_map_getter: Default::default(),
        root_cert_store: Default::default(),
        npm_resolver: Default::default(),
        blob_store: Default::default(),
        stdio: Default::default(),
    };

    // Build permissions object from extension's requested permissions.
    let permissions_options = PermissionsOptions::from(&*extension.permissions());
    let worker_permissions = Permissions::from_options(&permissions_options)?;

    // Initialize Deno runtime.
    let mut worker =
        MainWorker::bootstrap_from_options(main_module.clone(), worker_permissions, options);

    // Export shared state.
    let state = ExtensionState::new(api, extension);
    worker.js_runtime.op_state().borrow_mut().put(state);

    // Execute extension code.
    if let Err(error) = worker.execute_main_module(&main_module).await {
        return print_js_error(error);
    }
    if let Err(error) = worker.run_event_loop(false).await {
        return print_js_error(error);
    }

    Ok(ExitCode::Ok.into())
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

    println!("{}: {}", style("Extension error").red(), fmt_errors::format_js_error(&js_error));

    Ok(ExitCode::JsError.into())
}

/// See https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs.
struct ExtensionsModuleLoader {
    extension_path: Rc<PathBuf>,
}

impl ExtensionsModuleLoader {
    fn new(extension_path: PathBuf) -> Self {
        Self { extension_path: Rc::new(extension_path) }
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

    async fn load_from_deno_std(path: &Url) -> Result<String> {
        let response = reqwest::get(path.clone()).await?;
        Ok(response.text().await?)
    }
}

impl ModuleLoader for ExtensionsModuleLoader {
    fn resolve(&self, specifier: &str, referrer: &str, _is_main: bool) -> Result<ModuleSpecifier> {
        Ok(deno_core::resolve_import(specifier, referrer)?)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        let extension_path = self.extension_path.clone();

        Box::pin(async move {
            // Determine source file type.
            // We do not care about invalid URLs yet: This match statement is inexpensive,
            // bears no risk and does not do I/O -- it operates fully off of the
            // contents of the URL.
            let media_type = MediaType::from(&module_specifier);
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
                "https" => ExtensionsModuleLoader::load_from_deno_std(&module_specifier).await?,
                _ => return Err(anyhow!("Unsupported module specifier: {}", module_specifier)),
            };

            if should_transpile {
                code = transpile(module_specifier.to_string(), code, media_type)?;
            }

            Ok(ModuleSource {
                code: code.into_bytes().into_boxed_slice(),
                module_url_specified: module_specifier.to_string(),
                module_url_found: module_specifier.to_string(),
                module_type,
            })
        })
    }
}

/// Transpile code to JavaScript.
fn transpile(
    specifier: impl Into<String>,
    code: impl Into<String>,
    media_type: MediaType,
) -> Result<String> {
    let parsed = deno_ast::parse_module(ParseParams {
        text_info: SourceTextInfo::from_string(code.into()),
        specifier: specifier.into(),
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
        media_type,
    })?;
    Ok(parsed.transpile(&Default::default())?.text)
}
