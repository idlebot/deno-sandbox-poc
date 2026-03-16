use deno_core::error::ModuleLoaderError;
use deno_core::{
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    RequestedModuleType, ResolutionKind,
};
use deno_error::JsErrorBox;
use std::collections::HashMap;

pub struct AllowlistModuleLoader {
    modules: HashMap<String, String>,
}

impl AllowlistModuleLoader {
    pub fn new(modules: HashMap<String, String>) -> Self {
        Self { modules }
    }
}

impl ModuleLoader for AllowlistModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        _referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        if self.modules.contains_key(specifier) {
            let url = ModuleSpecifier::parse(specifier)
                .map_err(|e| ModuleLoaderError::from(JsErrorBox::generic(e.to_string())))?;
            return Ok(url);
        }

        Err(ModuleLoaderError::from(JsErrorBox::generic(format!(
            "module not allowed: {}",
            specifier
        ))))
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let specifier = module_specifier.as_str().to_string();

        if let Some(source) = self.modules.get(&specifier) {
            let module_source = ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(source.clone().into()),
                module_specifier,
                None,
            );
            ModuleLoadResponse::Sync(Ok(module_source))
        } else {
            ModuleLoadResponse::Sync(Err(ModuleLoaderError::from(JsErrorBox::generic(format!(
                "module not allowed: {}",
                specifier
            )))))
        }
    }
}
