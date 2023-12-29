// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use crate::args::npm_pkg_req_ref_to_binary_command;
use crate::args::CliOptions;
use crate::args::DenoSubcommand;
use crate::args::Flags;
use crate::args::Lockfile;
use crate::args::PackageJsonDepsProvider;
use crate::args::StorageKeyResolver;
use crate::args::TsConfigType;
use crate::cache::Caches;
use crate::cache::DenoDir;
use crate::cache::DenoDirProvider;
use crate::cache::EmitCache;
use crate::cache::GlobalHttpCache;
use crate::cache::HttpCache;
use crate::cache::LocalHttpCache;
use crate::cache::ModuleInfoCache;
use crate::cache::NodeAnalysisCache;
use crate::cache::ParsedSourceCache;
use crate::emit::Emitter;
use crate::file_fetcher::FileFetcher;
use crate::graph_util::FileWatcherReporter;
use crate::graph_util::ModuleGraphBuilder;
use crate::graph_util::ModuleGraphContainer;
use crate::http_util::HttpClient;
use crate::module_loader::CjsResolutionStore;
use crate::module_loader::CliModuleLoaderFactory;
use crate::module_loader::CliNodeResolver;
use crate::module_loader::ModuleLoadPreparer;
use crate::module_loader::NpmModuleLoader;
use crate::node::CliCjsCodeAnalyzer;
use crate::node::CliNodeCodeTranslator;
use crate::npm::create_cli_npm_resolver;
use crate::npm::CliNpmResolver;
use crate::npm::CliNpmResolverByonmCreateOptions;
use crate::npm::CliNpmResolverCreateOptions;
use crate::npm::CliNpmResolverManagedCreateOptions;
use crate::npm::CliNpmResolverManagedPackageJsonInstallerOption;
use crate::npm::CliNpmResolverManagedSnapshotOption;
use crate::resolver::CliGraphResolver;
use crate::resolver::CliGraphResolverOptions;
use crate::standalone::DenoCompileBinaryWriter;
use crate::tools::check::TypeChecker;
use crate::util::file_watcher::WatcherCommunicator;
use crate::util::progress_bar::ProgressBar;
use crate::util::progress_bar::ProgressBarStyle;
use crate::worker::CliMainWorkerFactory;
use crate::worker::CliMainWorkerOptions;

use deno_core::error::AnyError;
use deno_core::parking_lot::Mutex;
use deno_core::FeatureChecker;

use deno_graph::GraphKind;
use deno_runtime::deno_fs;
use deno_runtime::deno_node::analyze::NodeCodeTranslator;
use deno_runtime::deno_node::NodeResolver;
use deno_runtime::deno_tls::RootCertStoreProvider;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::inspector_server::InspectorServer;
use deno_semver::npm::NpmPackageReqReference;
use import_map::ImportMap;
use log::warn;
use std::future::Future;
use std::sync::Arc;

pub struct CliFactoryBuilder {
  watcher_communicator: Option<Arc<WatcherCommunicator>>,
}

impl CliFactoryBuilder {
  pub fn new() -> Self {
    Self { watcher_communicator: None }
  }

  pub async fn build_from_flags(self, flags: Flags) -> Result<CliFactory, AnyError> {
    Ok(self.build_from_cli_options(Arc::new(CliOptions::from_flags(flags)?)))
  }

  pub async fn build_from_flags_for_watcher(mut self, flags: Flags, watcher_communicator: Arc<WatcherCommunicator>) -> Result<CliFactory, AnyError> {
    self.watcher_communicator = Some(watcher_communicator);
    self.build_from_flags(flags).await
  }

  pub fn build_from_cli_options(self, options: Arc<CliOptions>) -> CliFactory {
    CliFactory {
      watcher_communicator: self.watcher_communicator,
      options,
      services: Default::default(),
    }
  }
}

struct Deferred<T>(once_cell::unsync::OnceCell<T>);

impl<T> Default for Deferred<T> {
  fn default() -> Self {
    Self(once_cell::unsync::OnceCell::default())
  }
}

impl<T> Deferred<T> {
  pub fn get_or_try_init(&self, create: impl FnOnce() -> Result<T, AnyError>) -> Result<&T, AnyError> {
    self.0.get_or_try_init(create)
  }

  pub fn get_or_init(&self, create: impl FnOnce() -> T) -> &T {
    self.0.get_or_init(create)
  }

  pub async fn get_or_try_init_async(&self, create: impl Future<Output = Result<T, AnyError>>) -> Result<&T, AnyError> {
    if self.0.get().is_none() {
      // todo(dsherret): it would be more ideal if this enforced a
      // single executor and then we could make some initialization
      // concurrent
      let val = create.await?;
      _ = self.0.set(val);
    }
    Ok(self.0.get().unwrap())
  }
}

#[derive(Default)]
struct CliFactoryServices {
  deno_dir_provider: Deferred<Arc<DenoDirProvider>>,
  caches: Deferred<Arc<Caches>>,
  file_fetcher: Deferred<Arc<FileFetcher>>,
  global_http_cache: Deferred<Arc<GlobalHttpCache>>,
  http_cache: Deferred<Arc<dyn HttpCache>>,
  http_client: Deferred<Arc<HttpClient>>,
  emit_cache: Deferred<EmitCache>,
  emitter: Deferred<Arc<Emitter>>,
  fs: Deferred<Arc<dyn deno_fs::FileSystem>>,
  graph_container: Deferred<Arc<ModuleGraphContainer>>,
  lockfile: Deferred<Option<Arc<Mutex<Lockfile>>>>,
  maybe_import_map: Deferred<Option<Arc<ImportMap>>>,
  maybe_inspector_server: Deferred<Option<Arc<InspectorServer>>>,
  root_cert_store_provider: Deferred<Arc<dyn RootCertStoreProvider>>,
  blob_store: Deferred<Arc<BlobStore>>,
  module_info_cache: Deferred<Arc<ModuleInfoCache>>,
  parsed_source_cache: Deferred<Arc<ParsedSourceCache>>,
  resolver: Deferred<Arc<CliGraphResolver>>,
  maybe_file_watcher_reporter: Deferred<Option<FileWatcherReporter>>,
  module_graph_builder: Deferred<Arc<ModuleGraphBuilder>>,
  module_load_preparer: Deferred<Arc<ModuleLoadPreparer>>,
  node_code_translator: Deferred<Arc<CliNodeCodeTranslator>>,
  node_resolver: Deferred<Arc<NodeResolver>>,
  npm_resolver: Deferred<Arc<dyn CliNpmResolver>>,
  package_json_deps_provider: Deferred<Arc<PackageJsonDepsProvider>>,
  text_only_progress_bar: Deferred<ProgressBar>,
  type_checker: Deferred<Arc<TypeChecker>>,
  cjs_resolutions: Deferred<Arc<CjsResolutionStore>>,
  cli_node_resolver: Deferred<Arc<CliNodeResolver>>,
  feature_checker: Deferred<Arc<FeatureChecker>>,
}

pub struct CliFactory {
  watcher_communicator: Option<Arc<WatcherCommunicator>>,
  options: Arc<CliOptions>,
  services: CliFactoryServices,
}

impl CliFactory {
  pub async fn from_flags(flags: Flags) -> Result<Self, AnyError> {
    CliFactoryBuilder::new().build_from_flags(flags).await
  }

  pub fn from_cli_options(options: Arc<CliOptions>) -> Self {
    CliFactoryBuilder::new().build_from_cli_options(options)
  }

  pub fn cli_options(&self) -> &Arc<CliOptions> {
    &self.options
  }

  pub fn deno_dir_provider(&self) -> &Arc<DenoDirProvider> {
    self.services.deno_dir_provider.get_or_init(|| Arc::new(DenoDirProvider::new(self.options.maybe_custom_root().clone())))
  }

  pub fn deno_dir(&self) -> Result<&DenoDir, AnyError> {
    Ok(self.deno_dir_provider().get_or_create()?)
  }

  pub fn caches(&self) -> Result<&Arc<Caches>, AnyError> {
    self.services.caches.get_or_try_init(|| {
      let caches = Arc::new(Caches::new(self.deno_dir_provider().clone()));
      // Warm up the caches we know we'll likely need based on the CLI mode
      match self.options.sub_command() {
        DenoSubcommand::Run(_) => {
          _ = caches.dep_analysis_db();
          _ = caches.node_analysis_db();
        }
        DenoSubcommand::Check(_) => {
          _ = caches.dep_analysis_db();
          _ = caches.node_analysis_db();
          _ = caches.type_checking_cache_db();
        }
        _ => {}
      }
      Ok(caches)
    })
  }

  pub fn blob_store(&self) -> &Arc<BlobStore> {
    self.services.blob_store.get_or_init(Default::default)
  }

  pub fn root_cert_store_provider(&self) -> &Arc<dyn RootCertStoreProvider> {
    self.services.root_cert_store_provider.get_or_init(|| self.options.resolve_root_cert_store_provider())
  }

  pub fn text_only_progress_bar(&self) -> &ProgressBar {
    self.services.text_only_progress_bar.get_or_init(|| ProgressBar::new(ProgressBarStyle::TextOnly))
  }

  pub fn global_http_cache(&self) -> Result<&Arc<GlobalHttpCache>, AnyError> {
    self
      .services
      .global_http_cache
      .get_or_try_init(|| Ok(Arc::new(GlobalHttpCache::new(self.deno_dir()?.deps_folder_path(), crate::cache::RealDenoCacheEnv))))
  }

  pub fn http_cache(&self) -> Result<&Arc<dyn HttpCache>, AnyError> {
    self.services.http_cache.get_or_try_init(|| {
      let global_cache = self.global_http_cache()?.clone();
      match self.options.vendor_dir_path() {
        Some(local_path) => {
          let local_cache = LocalHttpCache::new(local_path.clone(), global_cache);
          Ok(Arc::new(local_cache))
        }
        None => Ok(global_cache),
      }
    })
  }

  pub fn http_client(&self) -> &Arc<HttpClient> {
    self
      .services
      .http_client
      .get_or_init(|| Arc::new(HttpClient::new(Some(self.root_cert_store_provider().clone()), self.options.unsafely_ignore_certificate_errors().clone())))
  }

  pub fn file_fetcher(&self) -> Result<&Arc<FileFetcher>, AnyError> {
    self.services.file_fetcher.get_or_try_init(|| {
      Ok(Arc::new(FileFetcher::new(
        self.http_cache()?.clone(),
        self.options.cache_setting(),
        !self.options.no_remote(),
        self.http_client().clone(),
        self.blob_store().clone(),
        Some(self.text_only_progress_bar().clone()),
      )))
    })
  }

  pub fn fs(&self) -> &Arc<dyn deno_fs::FileSystem> {
    self.services.fs.get_or_init(|| Arc::new(deno_fs::RealFs))
  }

  pub fn maybe_lockfile(&self) -> &Option<Arc<Mutex<Lockfile>>> {
    self.services.lockfile.get_or_init(|| self.options.maybe_lockfile())
  }

  pub async fn npm_resolver(&self) -> Result<&Arc<dyn CliNpmResolver>, AnyError> {
    self
      .services
      .npm_resolver
      .get_or_try_init_async(async {
        let fs = self.fs();
        create_cli_npm_resolver(if self.options.unstable_byonm() {
          CliNpmResolverCreateOptions::Byonm(CliNpmResolverByonmCreateOptions {
            fs: fs.clone(),
            // todo(byonm): actually resolve this properly because the package.json
            // might be in an ancestor directory
            root_node_modules_dir: self.options.initial_cwd().join("node_modules"),
          })
        } else {
          CliNpmResolverCreateOptions::Managed(CliNpmResolverManagedCreateOptions {
            snapshot: match self.options.resolve_npm_resolution_snapshot()? {
              Some(snapshot) => CliNpmResolverManagedSnapshotOption::Specified(Some(snapshot)),
              None => match self.maybe_lockfile() {
                Some(lockfile) => CliNpmResolverManagedSnapshotOption::ResolveFromLockfile(lockfile.clone()),
                None => CliNpmResolverManagedSnapshotOption::Specified(None),
              },
            },
            maybe_lockfile: self.maybe_lockfile().as_ref().cloned(),
            fs: fs.clone(),
            http_client: self.http_client().clone(),
            npm_global_cache_dir: self.deno_dir()?.npm_folder_path(),
            cache_setting: self.options.cache_setting(),
            text_only_progress_bar: self.text_only_progress_bar().clone(),
            maybe_node_modules_path: self.options.node_modules_dir_path(),
            package_json_installer: CliNpmResolverManagedPackageJsonInstallerOption::ConditionalInstall(self.package_json_deps_provider().clone()),
            npm_system_info: self.options.npm_system_info(),
            npm_registry_url: crate::args::npm_registry_default_url().to_owned(),
          })
        })
        .await
      })
      .await
  }

  pub fn package_json_deps_provider(&self) -> &Arc<PackageJsonDepsProvider> {
    self.services.package_json_deps_provider.get_or_init(|| Arc::new(PackageJsonDepsProvider::new(self.options.maybe_package_json_deps())))
  }

  pub async fn maybe_import_map(&self) -> Result<&Option<Arc<ImportMap>>, AnyError> {
    self.services.maybe_import_map.get_or_try_init_async(async { Ok(self.options.resolve_import_map(self.file_fetcher()?).await?.map(Arc::new)) }).await
  }

  pub async fn resolver(&self) -> Result<&Arc<CliGraphResolver>, AnyError> {
    self
      .services
      .resolver
      .get_or_try_init_async(async {
        Ok(Arc::new(CliGraphResolver::new(CliGraphResolverOptions {
          fs: self.fs().clone(),
          cjs_resolutions: Some(self.cjs_resolutions().clone()),
          node_resolver: Some(self.node_resolver().await?.clone()),
          npm_resolver: if self.options.no_npm() { None } else { Some(self.npm_resolver().await?.clone()) },
          package_json_deps_provider: self.package_json_deps_provider().clone(),
          maybe_jsx_import_source_config: self.options.to_maybe_jsx_import_source_config()?,
          maybe_import_map: self.maybe_import_map().await?.clone(),
          maybe_vendor_dir: self.options.vendor_dir_path(),
          bare_node_builtins_enabled: self.options.unstable_bare_node_builtins(),
        })))
      })
      .await
  }

  pub fn maybe_file_watcher_reporter(&self) -> &Option<FileWatcherReporter> {
    let maybe_file_watcher_reporter = self.watcher_communicator.as_ref().map(|i| FileWatcherReporter::new(i.clone()));
    self.services.maybe_file_watcher_reporter.get_or_init(|| maybe_file_watcher_reporter)
  }

  pub fn emit_cache(&self) -> Result<&EmitCache, AnyError> {
    self.services.emit_cache.get_or_try_init(|| Ok(EmitCache::new(self.deno_dir()?.gen_cache.clone())))
  }

  pub fn module_info_cache(&self) -> Result<&Arc<ModuleInfoCache>, AnyError> {
    self.services.module_info_cache.get_or_try_init(|| Ok(Arc::new(ModuleInfoCache::new(self.caches()?.dep_analysis_db()))))
  }

  pub fn parsed_source_cache(&self) -> &Arc<ParsedSourceCache> {
    self.services.parsed_source_cache.get_or_init(Default::default)
  }

  pub fn emitter(&self) -> Result<&Arc<Emitter>, AnyError> {
    self.services.emitter.get_or_try_init(|| {
      let ts_config_result = self.options.resolve_ts_config_for_emit(TsConfigType::Emit)?;
      if let Some(ignored_options) = ts_config_result.maybe_ignored_options {
        warn!("{}", ignored_options);
      }
      let emit_options = crate::args::ts_config_to_emit_options(ts_config_result.ts_config);
      Ok(Arc::new(Emitter::new(self.emit_cache()?.clone(), self.parsed_source_cache().clone(), emit_options)))
    })
  }

  pub async fn node_resolver(&self) -> Result<&Arc<NodeResolver>, AnyError> {
    self
      .services
      .node_resolver
      .get_or_try_init_async(async { Ok(Arc::new(NodeResolver::new(self.fs().clone(), self.npm_resolver().await?.clone().into_npm_resolver()))) })
      .await
  }

  pub async fn node_code_translator(&self) -> Result<&Arc<CliNodeCodeTranslator>, AnyError> {
    self
      .services
      .node_code_translator
      .get_or_try_init_async(async {
        let caches = self.caches()?;
        let node_analysis_cache = NodeAnalysisCache::new(caches.node_analysis_db());
        let cjs_esm_analyzer = CliCjsCodeAnalyzer::new(node_analysis_cache, self.fs().clone());

        Ok(Arc::new(NodeCodeTranslator::new(
          cjs_esm_analyzer,
          self.fs().clone(),
          self.node_resolver().await?.clone(),
          self.npm_resolver().await?.clone().into_npm_resolver(),
        )))
      })
      .await
  }

  pub async fn type_checker(&self) -> Result<&Arc<TypeChecker>, AnyError> {
    self
      .services
      .type_checker
      .get_or_try_init_async(async { Ok(Arc::new(TypeChecker::new(self.caches()?.clone(), self.options.clone(), self.node_resolver().await?.clone(), self.npm_resolver().await?.clone()))) })
      .await
  }

  pub async fn module_graph_builder(&self) -> Result<&Arc<ModuleGraphBuilder>, AnyError> {
    self
      .services
      .module_graph_builder
      .get_or_try_init_async(async {
        Ok(Arc::new(ModuleGraphBuilder::new(
          self.options.clone(),
          self.resolver().await?.clone(),
          self.npm_resolver().await?.clone(),
          self.module_info_cache()?.clone(),
          self.parsed_source_cache().clone(),
          self.maybe_lockfile().clone(),
          self.maybe_file_watcher_reporter().clone(),
          self.emit_cache()?.clone(),
          self.file_fetcher()?.clone(),
          self.global_http_cache()?.clone(),
          self.type_checker().await?.clone(),
        )))
      })
      .await
  }

  pub fn graph_container(&self) -> &Arc<ModuleGraphContainer> {
    self.services.graph_container.get_or_init(|| {
      let graph_kind = match self.options.sub_command() {
        // todo(dsherret): ideally the graph container would not be used
        // for deno cache because it doesn't dynamically load modules
        DenoSubcommand::Cache(_) => GraphKind::All,
        _ => self.options.type_check_mode().as_graph_kind(),
      };
      Arc::new(ModuleGraphContainer::new(graph_kind))
    })
  }

  pub fn maybe_inspector_server(&self) -> &Option<Arc<InspectorServer>> {
    self.services.maybe_inspector_server.get_or_init(|| self.options.resolve_inspector_server().map(Arc::new))
  }

  pub async fn module_load_preparer(&self) -> Result<&Arc<ModuleLoadPreparer>, AnyError> {
    self
      .services
      .module_load_preparer
      .get_or_try_init_async(async {
        Ok(Arc::new(ModuleLoadPreparer::new(
          self.options.clone(),
          self.graph_container().clone(),
          self.maybe_lockfile().clone(),
          self.maybe_file_watcher_reporter().clone(),
          self.module_graph_builder().await?.clone(),
          self.module_info_cache()?.clone(),
          self.parsed_source_cache().clone(),
          self.text_only_progress_bar().clone(),
          self.resolver().await?.clone(),
          self.type_checker().await?.clone(),
        )))
      })
      .await
  }

  pub fn cjs_resolutions(&self) -> &Arc<CjsResolutionStore> {
    self.services.cjs_resolutions.get_or_init(Default::default)
  }

  pub async fn cli_node_resolver(&self) -> Result<&Arc<CliNodeResolver>, AnyError> {
    self
      .services
      .cli_node_resolver
      .get_or_try_init_async(async { Ok(Arc::new(CliNodeResolver::new(self.cjs_resolutions().clone(), self.node_resolver().await?.clone(), self.npm_resolver().await?.clone()))) })
      .await
  }

  pub fn feature_checker(&self) -> &Arc<FeatureChecker> {
    self.services.feature_checker.get_or_init(|| {
      let mut checker = FeatureChecker::default();
      checker.set_exit_cb(Box::new(crate::unstable_exit_cb));
      // TODO(bartlomieju): enable, once we deprecate `--unstable` in favor
      // of granular --unstable-* flags.
      // feature_checker.set_warn_cb(Box::new(crate::unstable_warn_cb));
      if self.options.unstable() {
        checker.enable_legacy_unstable();
      }
      let unstable_features = self.options.unstable_features();
      for (flag_name, _, _) in crate::UNSTABLE_GRANULAR_FLAGS {
        if unstable_features.contains(&flag_name.to_string()) {
          checker.enable_feature(flag_name);
        }
      }

      Arc::new(checker)
    })
  }

  pub async fn create_compile_binary_writer(&self) -> Result<DenoCompileBinaryWriter, AnyError> {
    Ok(DenoCompileBinaryWriter::new(
      self.file_fetcher()?,
      self.http_client(),
      self.deno_dir()?,
      self.npm_resolver().await?.as_ref(),
      self.options.npm_system_info(),
      self.package_json_deps_provider(),
    ))
  }

  pub async fn create_cli_main_worker_factory(&self) -> Result<CliMainWorkerFactory, AnyError> {
    let node_resolver = self.node_resolver().await?;
    let npm_resolver = self.npm_resolver().await?;
    let fs = self.fs();
    let cli_node_resolver = self.cli_node_resolver().await?;
    let maybe_file_watcher_communicator = if self.options.has_hmr() { Some(self.watcher_communicator.clone().unwrap()) } else { None };

    Ok(CliMainWorkerFactory::new(
      StorageKeyResolver::from_options(&self.options),
      self.options.sub_command().clone(),
      npm_resolver.clone(),
      node_resolver.clone(),
      self.blob_store().clone(),
      Box::new(CliModuleLoaderFactory::new(
        &self.options,
        self.emitter()?.clone(),
        self.graph_container().clone(),
        self.module_load_preparer().await?.clone(),
        self.parsed_source_cache().clone(),
        self.resolver().await?.clone(),
        cli_node_resolver.clone(),
        NpmModuleLoader::new(self.cjs_resolutions().clone(), self.node_code_translator().await?.clone(), fs.clone(), cli_node_resolver.clone()),
      )),
      self.root_cert_store_provider().clone(),
      self.fs().clone(),
      Some(self.emitter()?.clone()),
      maybe_file_watcher_communicator,
      self.maybe_inspector_server().clone(),
      self.maybe_lockfile().clone(),
      self.feature_checker().clone(),
      self.create_cli_main_worker_options()?,
    ))
  }

  fn create_cli_main_worker_options(&self) -> Result<CliMainWorkerOptions, AnyError> {
    Ok(CliMainWorkerOptions {
      argv: self.options.argv().clone(),
      // This optimization is only available for "run" subcommand
      // because we need to register new ops for testing and jupyter
      // integration.
      skip_op_registration: self.options.sub_command().is_run(),
      log_level: self.options.log_level().unwrap_or(log::Level::Info).into(),
      coverage_dir: self.options.coverage_dir(),
      enable_op_summary_metrics: self.options.enable_op_summary_metrics(),
      enable_testing_features: self.options.enable_testing_features(),
      has_node_modules_dir: self.options.has_node_modules_dir(),
      hmr: self.options.has_hmr(),
      inspect_brk: self.options.inspect_brk().is_some(),
      inspect_wait: self.options.inspect_wait().is_some(),
      strace_ops: self.options.strace_ops().clone(),
      is_inspecting: self.options.is_inspecting(),
      is_npm_main: self.options.is_npm_main(),
      location: self.options.location_flag().clone(),
      maybe_binary_npm_command_name: {
        let mut maybe_binary_command_name = None;
        if let DenoSubcommand::Run(flags) = self.options.sub_command() {
          if let Ok(pkg_ref) = NpmPackageReqReference::from_str(&flags.script) {
            // if the user ran a binary command, we'll need to set process.argv[0]
            // to be the name of the binary command instead of deno
            maybe_binary_command_name = Some(npm_pkg_req_ref_to_binary_command(&pkg_ref));
          }
        }
        maybe_binary_command_name
      },
      origin_data_folder_path: Some(self.deno_dir()?.origin_data_folder_path()),
      seed: self.options.seed(),
      unsafely_ignore_certificate_errors: self.options.unsafely_ignore_certificate_errors().clone(),
      unstable: self.options.unstable(),
      maybe_root_package_json_deps: self.options.maybe_package_json_deps(),
    })
  }
}
