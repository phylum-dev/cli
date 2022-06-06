//! Deno runtime for extensions.

use std::fs;
use std::pin::Pin;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_core::{
    Extension, JsRuntime, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier,
    ModuleType, RuntimeOptions,
};

use crate::commands::extensions::{api_decls, ExtensionState};

/// Deno runtime state.
pub struct DenoRuntime {
    runtime: JsRuntime,
}

impl DenoRuntime {
    /// Create a new Deno runtime.
    pub fn new(deps: ExtensionState) -> Self {
        let phylum_api = Extension::builder()
            .ops(api_decls())
            .build();

        let mut runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(TypescriptModuleLoader)),
            extensions: vec![phylum_api],
            ..Default::default()
        });

        let op_state = runtime.op_state();
        let mut op_state = op_state.borrow_mut();
        op_state.put(deps);

        Self { runtime }
    }

    /// Execute a JavaScript module from its main entry point.
    pub async fn run(&mut self, entrypoint: &str) -> Result<()> {
        let module_specifier = deno_core::resolve_path(entrypoint)?;
        let module = self
            .runtime
            .load_main_module(&module_specifier, None)
            .await?;
        let _ = self.runtime.mod_evaluate(module);

        self.runtime.run_event_loop(false).await?;

        Ok(())
    }
}

/// See https://github.com/denoland/deno/blob/main/core/examples/ts_module_loader.rs.
struct TypescriptModuleLoader;

impl ModuleLoader for TypescriptModuleLoader {
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
        Box::pin(async move {
            let path = module_specifier
                .to_file_path()
                .map_err(|_| anyhow!("Invalid module path"))?;

            // Determine source file type.
            let media_type = MediaType::from(&path);
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (ModuleType::JavaScript, false)
                }
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

            // Read the source and transpile it if necessary.
            let mut code = fs::read_to_string(&path)?;
            if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.to_string(),
                    text_info: SourceTextInfo::from_string(code),
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                    media_type,
                })?;
                code = parsed.transpile(&Default::default())?.text;
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
