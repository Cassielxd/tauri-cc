// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use deno_core::anyhow::anyhow;
use deno_core::error::AnyError;
use deno_core::futures::future;
use deno_core::futures::future::LocalBoxFuture;
use deno_core::futures::FutureExt;
use deno_core::ModuleSpecifier;
use deno_graph::source::NpmPackageReqResolution;
use deno_graph::source::NpmResolver;
use deno_graph::source::ResolutionMode;
use deno_graph::source::ResolveError;
use deno_graph::source::Resolver;
use deno_graph::source::UnknownBuiltInNodeModuleError;
use deno_graph::source::DEFAULT_JSX_IMPORT_SOURCE_MODULE;
use deno_runtime::deno_fs::FileSystem;
use deno_runtime::deno_node::is_builtin_node_module;
use deno_runtime::deno_node::parse_npm_pkg_name;
use deno_runtime::deno_node::NodeResolution;
use deno_runtime::deno_node::NodeResolutionMode;
use deno_runtime::deno_node::NodeResolver;
use deno_runtime::deno_node::NpmResolver as DenoNodeNpmResolver;
use deno_runtime::permissions::PermissionsContainer;
use deno_semver::npm::NpmPackageReqReference;
use deno_semver::package::PackageReq;
use import_map::ImportMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::args::package_json::PackageJsonDeps;
use crate::args::JsxImportSourceConfig;
use crate::args::PackageJsonDepsProvider;
use crate::module_loader::CjsResolutionStore;
use crate::npm::ByonmCliNpmResolver;
use crate::npm::CliNpmResolver;
use crate::npm::InnerCliNpmResolverRef;
use crate::util::sync::AtomicFlag;

/// Result of checking if a specifier is mapped via
/// an import map or package.json.
pub enum MappedResolution {
  None,
  PackageJson(ModuleSpecifier),
  ImportMap(ModuleSpecifier),
}

impl MappedResolution {
  pub fn into_specifier(self) -> Option<ModuleSpecifier> {
    match self {
      MappedResolution::None => Option::None,
      MappedResolution::PackageJson(specifier) => Some(specifier),
      MappedResolution::ImportMap(specifier) => Some(specifier),
    }
  }
}

/// Resolver for specifiers that could be mapped via an
/// import map or package.json.
#[derive(Debug)]
pub struct MappedSpecifierResolver {
  maybe_import_map: Option<Arc<ImportMap>>,
  package_json_deps_provider: Arc<PackageJsonDepsProvider>,
}

impl MappedSpecifierResolver {
  pub fn new(maybe_import_map: Option<Arc<ImportMap>>, package_json_deps_provider: Arc<PackageJsonDepsProvider>) -> Self {
    Self { maybe_import_map, package_json_deps_provider }
  }

  pub fn resolve(&self, specifier: &str, referrer: &ModuleSpecifier) -> Result<MappedResolution, AnyError> {
    // attempt to resolve with the import map first
    let maybe_import_map_err = match self.maybe_import_map.as_ref().map(|import_map| import_map.resolve(specifier, referrer)) {
      Some(Ok(value)) => return Ok(MappedResolution::ImportMap(value)),
      Some(Err(err)) => Some(err),
      None => None,
    };

    // then with package.json
    if let Some(deps) = self.package_json_deps_provider.deps() {
      if let Some(specifier) = resolve_package_json_dep(specifier, deps)? {
        return Ok(MappedResolution::PackageJson(specifier));
      }
    }

    // otherwise, surface the import map error or try resolving when has no import map
    if let Some(err) = maybe_import_map_err {
      Err(err.into())
    } else {
      Ok(MappedResolution::None)
    }
  }
}

/// A resolver that takes care of resolution, taking into account loaded
/// import map, JSX settings.
#[derive(Debug)]
pub struct CliGraphResolver {
  fs: Arc<dyn FileSystem>,
  mapped_specifier_resolver: MappedSpecifierResolver,
  maybe_default_jsx_import_source: Option<String>,
  maybe_jsx_import_source_module: Option<String>,
  maybe_vendor_specifier: Option<ModuleSpecifier>,
  cjs_resolutions: Option<Arc<CjsResolutionStore>>,
  node_resolver: Option<Arc<NodeResolver>>,
  npm_resolver: Option<Arc<dyn CliNpmResolver>>,
  found_package_json_dep_flag: Arc<AtomicFlag>,
  bare_node_builtins_enabled: bool,
}

pub struct CliGraphResolverOptions<'a> {
  pub fs: Arc<dyn FileSystem>,
  pub cjs_resolutions: Option<Arc<CjsResolutionStore>>,
  pub node_resolver: Option<Arc<NodeResolver>>,
  pub npm_resolver: Option<Arc<dyn CliNpmResolver>>,
  pub package_json_deps_provider: Arc<PackageJsonDepsProvider>,
  pub maybe_jsx_import_source_config: Option<JsxImportSourceConfig>,
  pub maybe_import_map: Option<Arc<ImportMap>>,
  pub maybe_vendor_dir: Option<&'a PathBuf>,
  pub bare_node_builtins_enabled: bool,
}

impl CliGraphResolver {
  pub fn new(options: CliGraphResolverOptions) -> Self {
    let is_byonm = options.npm_resolver.as_ref().map(|n| n.as_byonm().is_some()).unwrap_or(false);
    Self {
      fs: options.fs,
      cjs_resolutions: options.cjs_resolutions,
      mapped_specifier_resolver: MappedSpecifierResolver::new(
        options.maybe_import_map,
        if is_byonm {
          // don't resolve from the root package.json deps for byonm
          Arc::new(PackageJsonDepsProvider::new(None))
        } else {
          options.package_json_deps_provider
        },
      ),
      maybe_default_jsx_import_source: options.maybe_jsx_import_source_config.as_ref().and_then(|c| c.default_specifier.clone()),
      maybe_jsx_import_source_module: options.maybe_jsx_import_source_config.map(|c| c.module),
      maybe_vendor_specifier: options.maybe_vendor_dir.and_then(|v| ModuleSpecifier::from_directory_path(v).ok()),
      node_resolver: options.node_resolver,
      npm_resolver: options.npm_resolver,
      found_package_json_dep_flag: Default::default(),
      bare_node_builtins_enabled: options.bare_node_builtins_enabled,
    }
  }

  pub fn as_graph_resolver(&self) -> &dyn Resolver {
    self
  }

  pub fn as_graph_npm_resolver(&self) -> &dyn NpmResolver {
    self
  }

  pub fn found_package_json_dep(&self) -> bool {
    self.found_package_json_dep_flag.is_raised()
  }

  fn check_surface_byonm_node_error(&self, specifier: &str, referrer: &ModuleSpecifier, mode: NodeResolutionMode, original_err: AnyError, resolver: &ByonmCliNpmResolver) -> Result<(), AnyError> {
    if let Ok((pkg_name, _, _)) = parse_npm_pkg_name(specifier, referrer) {
      match resolver.resolve_package_folder_from_package(&pkg_name, referrer, mode) {
        Ok(_) => {
          return Err(original_err);
        }
        Err(_) => {
          if resolver.find_ancestor_package_json_with_dep(&pkg_name, referrer).is_some() {
            return Err(anyhow!(
              concat!(
                "Could not resolve \"{}\", but found it in a package.json. ",
                "Deno expects the node_modules/ directory to be up to date. ",
                "Did you forget to run `npm install`?"
              ),
              specifier
            ));
          }
        }
      }
    }
    Ok(())
  }
}

impl Resolver for CliGraphResolver {
  fn default_jsx_import_source(&self) -> Option<String> {
    self.maybe_default_jsx_import_source.clone()
  }

  fn jsx_import_source_module(&self) -> &str {
    self.maybe_jsx_import_source_module.as_deref().unwrap_or(DEFAULT_JSX_IMPORT_SOURCE_MODULE)
  }

  fn resolve(&self, specifier: &str, referrer: &ModuleSpecifier, mode: ResolutionMode) -> Result<ModuleSpecifier, ResolveError> {
    fn to_node_mode(mode: ResolutionMode) -> NodeResolutionMode {
      match mode {
        ResolutionMode::Execution => NodeResolutionMode::Execution,
        ResolutionMode::Types => NodeResolutionMode::Types,
      }
    }

    let result = match self.mapped_specifier_resolver.resolve(specifier, referrer)? {
      MappedResolution::ImportMap(specifier) => Ok(specifier),
      MappedResolution::PackageJson(specifier) => {
        // found a specifier in the package.json, so mark that
        // we need to do an "npm install" later
        self.found_package_json_dep_flag.raise();
        Ok(specifier)
      }
      MappedResolution::None => deno_graph::resolve_import(specifier, referrer).map_err(|err| err.into()),
    };

    // When the user is vendoring, don't allow them to import directly from the vendor/ directory
    // as it might cause them confusion or duplicate dependencies. Additionally, this folder has
    // special treatment in the language server so it will definitely cause issues/confusion there
    // if they do this.
    if let Some(vendor_specifier) = &self.maybe_vendor_specifier {
      if let Ok(specifier) = &result {
        if specifier.as_str().starts_with(vendor_specifier.as_str()) {
          return Err(ResolveError::Other(anyhow!("Importing from the vendor directory is not permitted. Use a remote specifier instead or disable vendoring.")));
        }
      }
    }

    if let Some(resolver) = self.npm_resolver.as_ref().and_then(|r| r.as_byonm()) {
      match &result {
        Ok(specifier) => {
          if let Ok(npm_req_ref) = NpmPackageReqReference::from_specifier(specifier) {
            let package_folder = resolver.resolve_pkg_folder_from_deno_module_req(npm_req_ref.req(), referrer)?;
            let node_resolver = self.node_resolver.as_ref().unwrap();
            let package_json_path = package_folder.join("package.json");
            if !self.fs.exists_sync(&package_json_path) {
              return Err(ResolveError::Other(anyhow!(
                "Could not find '{}'. Deno expects the node_modules/ directory to be up to date. Did you forget to run `npm install`?",
                package_json_path.display()
              )));
            }
            let maybe_resolution = node_resolver.resolve_package_subpath_from_deno_module(&package_folder, npm_req_ref.sub_path(), referrer, to_node_mode(mode), &PermissionsContainer::allow_all())?;
            match maybe_resolution {
              Some(resolution) => {
                if let Some(cjs_resolutions) = &self.cjs_resolutions {
                  if let NodeResolution::CommonJs(specifier) = &resolution {
                    // remember that this was a common js resolution
                    cjs_resolutions.insert(specifier.clone());
                  }
                }

                return Ok(resolution.into_url());
              }
              None => {
                return Err(ResolveError::Other(anyhow!("Failed resolving package subpath for '{}' in '{}'.", npm_req_ref, package_folder.display())));
              }
            }
          }
        }
        Err(_) => {
          if referrer.scheme() == "file" {
            if let Some(node_resolver) = &self.node_resolver {
              let node_result = node_resolver.resolve(specifier, referrer, to_node_mode(mode), &PermissionsContainer::allow_all());
              match node_result {
                Ok(Some(resolution)) => {
                  if let Some(cjs_resolutions) = &self.cjs_resolutions {
                    if let NodeResolution::CommonJs(specifier) = &resolution {
                      // remember that this was a common js resolution
                      cjs_resolutions.insert(specifier.clone());
                    }
                  }
                  return Ok(resolution.into_url());
                }
                Ok(None) => {
                  self
                    .check_surface_byonm_node_error(specifier, referrer, to_node_mode(mode), anyhow!("Cannot find \"{}\"", specifier), resolver)
                    .map_err(ResolveError::Other)?;
                }
                Err(err) => {
                  self.check_surface_byonm_node_error(specifier, referrer, to_node_mode(mode), err, resolver).map_err(ResolveError::Other)?;
                }
              }
            }
          }
        }
      }
    }

    result
  }
}

fn resolve_package_json_dep(specifier: &str, deps: &PackageJsonDeps) -> Result<Option<ModuleSpecifier>, AnyError> {
  for (bare_specifier, req_result) in deps {
    if specifier.starts_with(bare_specifier) {
      let path = &specifier[bare_specifier.len()..];
      if path.is_empty() || path.starts_with('/') {
        let req = req_result
          .as_ref()
          .map_err(|err| anyhow!("Parsing version constraints in the application-level package.json is more strict at the moment.\n\n{:#}", err.clone()))?;
        return Ok(Some(ModuleSpecifier::parse(&format!("npm:{req}{path}"))?));
      }
    }
  }

  Ok(None)
}

impl NpmResolver for CliGraphResolver {
  fn resolve_builtin_node_module(&self, specifier: &ModuleSpecifier) -> Result<Option<String>, UnknownBuiltInNodeModuleError> {
    if specifier.scheme() != "node" {
      return Ok(None);
    }

    let module_name = specifier.path().to_string();
    if is_builtin_node_module(&module_name) {
      Ok(Some(module_name))
    } else {
      Err(UnknownBuiltInNodeModuleError { module_name })
    }
  }

  fn on_resolve_bare_builtin_node_module(&self, module_name: &str, range: &deno_graph::Range) {
    let deno_graph::Range { start, specifier, .. } = range;
    let line = start.line + 1;
    let column = start.character + 1;
    log::warn!("Warning: Resolving \"{module_name}\" as \"node:{module_name}\" at {specifier}:{line}:{column}. If you want to use a built-in Node module, add a \"node:\" prefix.")
  }

  fn load_and_cache_npm_package_info(&self, package_name: &str) -> LocalBoxFuture<'static, Result<(), AnyError>> {
    match &self.npm_resolver {
      Some(npm_resolver) if npm_resolver.as_managed().is_some() => {
        let package_name = package_name.to_string();
        let npm_resolver = npm_resolver.clone();
        async move {
          if let Some(managed) = npm_resolver.as_managed() {
            managed.cache_package_info(&package_name).await?;
          }
          Ok(())
        }
        .boxed()
      }
      _ => {
        // return it succeeded and error at the import site below
        Box::pin(future::ready(Ok(())))
      }
    }
  }

  fn resolve_npm(&self, package_req: &PackageReq) -> NpmPackageReqResolution {
    match &self.npm_resolver {
      Some(npm_resolver) => match npm_resolver.as_inner() {
        InnerCliNpmResolverRef::Managed(npm_resolver) => npm_resolver.resolve_npm_for_deno_graph(package_req),
        // if we are using byonm, then this should never be called because
        // we don't use deno_graph's npm resolution in this case
        InnerCliNpmResolverRef::Byonm(_) => unreachable!(),
      },
      None => NpmPackageReqResolution::Err(anyhow!("npm specifiers were requested; but --no-npm is specified")),
    }
  }

  fn enables_bare_builtin_node_module(&self) -> bool {
    self.bare_node_builtins_enabled
  }
}


