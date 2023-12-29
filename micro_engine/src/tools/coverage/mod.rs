// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use crate::args::CoverageFlags;
use crate::args::FileFlags;
use crate::args::Flags;
use crate::cdp;
use crate::colors;
use crate::factory::CliFactory;
use crate::npm::CliNpmResolver;
use crate::tools::fmt::format_json;
use crate::tools::test::is_supported_test_path;
use crate::util::fs::FileCollector;
use crate::util::text_encoding::source_map_from_code;

use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::Context;
use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::sourcemap::SourceMap;
use deno_core::url::Url;
use deno_core::LocalInspectorSession;
use deno_core::ModuleCode;
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Error;
use std::io::Write;
use std::io::{self};
use std::path::Path;
use std::path::PathBuf;
use text_lines::TextLines;
use uuid::Uuid;

mod merge;
mod range_tree;
use merge::ProcessCoverage;

pub struct CoverageCollector {
  pub dir: PathBuf,
  session: LocalInspectorSession,
}

impl CoverageCollector {
  pub fn new(dir: PathBuf, session: LocalInspectorSession) -> Self {
    Self { dir, session }
  }

  async fn enable_debugger(&mut self) -> Result<(), AnyError> {
    self.session.post_message::<()>("Debugger.enable", None).await?;
    Ok(())
  }

  async fn enable_profiler(&mut self) -> Result<(), AnyError> {
    self.session.post_message::<()>("Profiler.enable", None).await?;
    Ok(())
  }

  async fn disable_debugger(&mut self) -> Result<(), AnyError> {
    self.session.post_message::<()>("Debugger.disable", None).await?;
    Ok(())
  }

  async fn disable_profiler(&mut self) -> Result<(), AnyError> {
    self.session.post_message::<()>("Profiler.disable", None).await?;
    Ok(())
  }

  async fn start_precise_coverage(&mut self, parameters: cdp::StartPreciseCoverageArgs) -> Result<cdp::StartPreciseCoverageResponse, AnyError> {
    let return_value = self.session.post_message("Profiler.startPreciseCoverage", Some(parameters)).await?;

    let return_object = serde_json::from_value(return_value)?;

    Ok(return_object)
  }

  async fn take_precise_coverage(&mut self) -> Result<cdp::TakePreciseCoverageResponse, AnyError> {
    let return_value = self.session.post_message::<()>("Profiler.takePreciseCoverage", None).await?;

    let return_object = serde_json::from_value(return_value)?;

    Ok(return_object)
  }

  pub async fn start_collecting(&mut self) -> Result<(), AnyError> {
    self.enable_debugger().await?;
    self.enable_profiler().await?;
    self
      .start_precise_coverage(cdp::StartPreciseCoverageArgs {
        call_count: true,
        detailed: true,
        allow_triggered_updates: false,
      })
      .await?;

    Ok(())
  }

  pub async fn stop_collecting(&mut self) -> Result<(), AnyError> {
    fs::create_dir_all(&self.dir)?;

    let script_coverages = self.take_precise_coverage().await?.result;
    for script_coverage in script_coverages {
      // Filter out internal JS files from being included in coverage reports
      if script_coverage.url.starts_with("ext:") || script_coverage.url.starts_with("[ext:") {
        continue;
      }

      let filename = format!("{}.json", Uuid::new_v4());
      let filepath = self.dir.join(filename);

      let mut out = BufWriter::new(File::create(&filepath)?);
      let coverage = serde_json::to_string(&script_coverage)?;
      let formatted_coverage = format_json(&filepath, &coverage, &Default::default()).ok().flatten().unwrap_or(coverage);

      out.write_all(formatted_coverage.as_bytes())?;
      out.flush()?;
    }

    self.disable_debugger().await?;
    self.disable_profiler().await?;

    Ok(())
  }
}

struct BranchCoverageItem {
  line_index: usize,
  block_number: usize,
  branch_number: usize,
  taken: Option<i64>,
  is_hit: bool,
}

struct FunctionCoverageItem {
  name: String,
  line_index: usize,
  execution_count: i64,
}

struct CoverageReport {
  url: ModuleSpecifier,
  named_functions: Vec<FunctionCoverageItem>,
  branches: Vec<BranchCoverageItem>,
  found_lines: Vec<(usize, i64)>,
  output: Option<PathBuf>,
}

fn generate_coverage_report(script_coverage: &cdp::ScriptCoverage, script_source: String, maybe_source_map: &Option<Vec<u8>>, output: &Option<PathBuf>) -> CoverageReport {
  let maybe_source_map = maybe_source_map.as_ref().map(|source_map| SourceMap::from_slice(source_map).unwrap());
  let text_lines = TextLines::new(&script_source);

  let comment_ranges = deno_ast::lex(&script_source, MediaType::JavaScript)
    .into_iter()
    .filter(|item| matches!(item.inner, deno_ast::TokenOrComment::Comment { .. }))
    .map(|item| item.range)
    .collect::<Vec<_>>();

  let url = Url::parse(&script_coverage.url).unwrap();
  let mut coverage_report = CoverageReport {
    url,
    named_functions: Vec::with_capacity(script_coverage.functions.iter().filter(|f| !f.function_name.is_empty()).count()),
    branches: Vec::new(),
    found_lines: Vec::new(),
    output: output.clone(),
  };

  for function in &script_coverage.functions {
    if function.function_name.is_empty() {
      continue;
    }

    let dest_line_index = text_lines.line_index(text_lines.byte_index_from_char_index(function.ranges[0].start_char_offset));
    let line_index = if let Some(source_map) = maybe_source_map.as_ref() {
      source_map.tokens().find(|token| token.get_dst_line() as usize == dest_line_index).map(|token| token.get_src_line() as usize).unwrap_or(0)
    } else {
      dest_line_index
    };

    coverage_report.named_functions.push(FunctionCoverageItem {
      name: function.function_name.clone(),
      line_index,
      execution_count: function.ranges[0].count,
    });
  }

  for (block_number, function) in script_coverage.functions.iter().enumerate() {
    let block_hits = function.ranges[0].count;
    for (branch_number, range) in function.ranges[1..].iter().enumerate() {
      let source_line_index = text_lines.line_index(text_lines.byte_index_from_char_index(range.start_char_offset));
      let line_index = if let Some(source_map) = maybe_source_map.as_ref() {
        source_map.tokens().find(|token| token.get_dst_line() as usize == source_line_index).map(|token| token.get_src_line() as usize).unwrap_or(0)
      } else {
        source_line_index
      };

      // From https://manpages.debian.org/unstable/lcov/geninfo.1.en.html:
      //
      // Block number and branch number are gcc internal IDs for the branch. Taken is either '-'
      // if the basic block containing the branch was never executed or a number indicating how
      // often that branch was taken.
      //
      // However with the data we get from v8 coverage profiles it seems we can't actually hit
      // this as appears it won't consider any nested branches it hasn't seen but its here for
      // the sake of accuracy.
      let taken = if block_hits > 0 { Some(range.count) } else { None };

      coverage_report.branches.push(BranchCoverageItem {
        line_index,
        block_number,
        branch_number,
        taken,
        is_hit: range.count > 0,
      })
    }
  }

  // TODO(caspervonb): collect uncovered ranges on the lines so that we can highlight specific
  // parts of a line in color (word diff style) instead of the entire line.
  let mut line_counts = Vec::with_capacity(text_lines.lines_count());
  for line_index in 0..text_lines.lines_count() {
    let line_start_byte_offset = text_lines.line_start(line_index);
    let line_start_char_offset = text_lines.char_index(line_start_byte_offset);
    let line_end_byte_offset = text_lines.line_end(line_index);
    let line_end_char_offset = text_lines.char_index(line_end_byte_offset);
    let ignore = comment_ranges.iter().any(|range| range.start <= line_start_byte_offset && range.end >= line_end_byte_offset) || script_source[line_start_byte_offset..line_end_byte_offset].trim().is_empty();
    let mut count = 0;

    if ignore {
      count = 1;
    } else {
      // Count the hits of ranges that include the entire line which will always be at-least one
      // as long as the code has been evaluated.
      for function in &script_coverage.functions {
        for range in &function.ranges {
          if range.start_char_offset <= line_start_char_offset && range.end_char_offset >= line_end_char_offset {
            count += range.count;
          }
        }
      }

      // We reset the count if any block with a zero count overlaps with the line range.
      for function in &script_coverage.functions {
        for range in &function.ranges {
          if range.count > 0 {
            continue;
          }

          let overlaps = range.start_char_offset < line_end_char_offset && range.end_char_offset > line_start_char_offset;
          if overlaps {
            count = 0;
          }
        }
      }
    }

    line_counts.push(count);
  }

  coverage_report.found_lines = if let Some(source_map) = maybe_source_map.as_ref() {
    let mut found_lines = line_counts
      .iter()
      .enumerate()
      .flat_map(|(index, count)| {
        // get all the mappings from this destination line to a different src line
        let mut results = source_map
          .tokens()
          .filter(move |token| token.get_dst_line() as usize == index)
          .map(move |token| (token.get_src_line() as usize, *count))
          .collect::<Vec<_>>();
        // only keep the results that point at different src lines
        results.sort_unstable_by_key(|(index, _)| *index);
        results.dedup_by_key(|(index, _)| *index);
        results.into_iter()
      })
      .collect::<Vec<(usize, i64)>>();

    found_lines.sort_unstable_by_key(|(index, _)| *index);
    // combine duplicated lines
    for i in (1..found_lines.len()).rev() {
      if found_lines[i].0 == found_lines[i - 1].0 {
        found_lines[i - 1].1 += found_lines[i].1;
        found_lines.remove(i);
      }
    }
    found_lines
  } else {
    line_counts.into_iter().enumerate().map(|(index, count)| (index, count)).collect::<Vec<(usize, i64)>>()
  };

  coverage_report
}

enum CoverageReporterKind {
  Pretty,
  Lcov,
}

fn create_reporter(kind: CoverageReporterKind) -> Box<dyn CoverageReporter + Send> {
  match kind {
    CoverageReporterKind::Lcov => Box::new(LcovCoverageReporter::new()),
    CoverageReporterKind::Pretty => Box::new(PrettyCoverageReporter::new()),
  }
}

trait CoverageReporter {
  fn report(&mut self, coverage_report: &CoverageReport, file_text: &str) -> Result<(), AnyError>;

  fn done(&mut self);
}

struct LcovCoverageReporter {}

impl LcovCoverageReporter {
  pub fn new() -> LcovCoverageReporter {
    LcovCoverageReporter {}
  }
}

impl CoverageReporter for LcovCoverageReporter {
  fn report(&mut self, coverage_report: &CoverageReport, _file_text: &str) -> Result<(), AnyError> {
    // pipes output to stdout if no file is specified
    let out_mode: Result<Box<dyn Write>, Error> = match coverage_report.output {
      // only append to the file as the file should be created already
      Some(ref path) => File::options().append(true).open(path).map(|f| Box::new(f) as Box<dyn Write>),
      None => Ok(Box::new(io::stdout())),
    };
    let mut out_writer = out_mode?;

    let file_path = coverage_report.url.to_file_path().ok().and_then(|p| p.to_str().map(|p| p.to_string())).unwrap_or_else(|| coverage_report.url.to_string());
    writeln!(out_writer, "SF:{file_path}")?;

    for function in &coverage_report.named_functions {
      writeln!(out_writer, "FN:{},{}", function.line_index + 1, function.name)?;
    }

    for function in &coverage_report.named_functions {
      writeln!(out_writer, "FNDA:{},{}", function.execution_count, function.name)?;
    }

    let functions_found = coverage_report.named_functions.len();
    writeln!(out_writer, "FNF:{functions_found}")?;
    let functions_hit = coverage_report.named_functions.iter().filter(|f| f.execution_count > 0).count();
    writeln!(out_writer, "FNH:{functions_hit}")?;

    for branch in &coverage_report.branches {
      let taken = if let Some(taken) = &branch.taken { taken.to_string() } else { "-".to_string() };

      writeln!(out_writer, "BRDA:{},{},{},{}", branch.line_index + 1, branch.block_number, branch.branch_number, taken)?;
    }

    let branches_found = coverage_report.branches.len();
    writeln!(out_writer, "BRF:{branches_found}")?;
    let branches_hit = coverage_report.branches.iter().filter(|b| b.is_hit).count();
    writeln!(out_writer, "BRH:{branches_hit}")?;
    for (index, count) in &coverage_report.found_lines {
      writeln!(out_writer, "DA:{},{}", index + 1, count)?;
    }

    let lines_hit = coverage_report.found_lines.iter().filter(|(_, count)| *count != 0).count();
    writeln!(out_writer, "LH:{lines_hit}")?;

    let lines_found = coverage_report.found_lines.len();
    writeln!(out_writer, "LF:{lines_found}")?;

    writeln!(out_writer, "end_of_record")?;
    Ok(())
  }

  fn done(&mut self) {}
}

struct PrettyCoverageReporter {}

impl PrettyCoverageReporter {
  pub fn new() -> PrettyCoverageReporter {
    PrettyCoverageReporter {}
  }
}

impl CoverageReporter for PrettyCoverageReporter {
  fn report(&mut self, coverage_report: &CoverageReport, file_text: &str) -> Result<(), AnyError> {
    let lines = file_text.split('\n').collect::<Vec<_>>();
    print!("cover {} ... ", coverage_report.url);

    let hit_lines = coverage_report.found_lines.iter().filter(|(_, count)| *count > 0).map(|(index, _)| *index);

    let missed_lines = coverage_report.found_lines.iter().filter(|(_, count)| *count == 0).map(|(index, _)| *index);

    let lines_found = coverage_report.found_lines.len();
    let lines_hit = hit_lines.count();
    let line_ratio = lines_hit as f32 / lines_found as f32;

    let line_coverage = format!("{:.3}% ({}/{})", line_ratio * 100.0, lines_hit, lines_found);

    if line_ratio >= 0.9 {
      println!("{}", colors::green(&line_coverage));
    } else if line_ratio >= 0.75 {
      println!("{}", colors::yellow(&line_coverage));
    } else {
      println!("{}", colors::red(&line_coverage));
    }

    let mut last_line = None;
    for line_index in missed_lines {
      const WIDTH: usize = 4;
      const SEPARATOR: &str = "|";

      // Put a horizontal separator between disjoint runs of lines
      if let Some(last_line) = last_line {
        if last_line + 1 != line_index {
          let dash = colors::gray("-".repeat(WIDTH + 1));
          println!("{}{}{}", dash, colors::gray(SEPARATOR), dash);
        }
      }

      println!("{:width$} {} {}", line_index + 1, colors::gray(SEPARATOR), colors::red(&lines[line_index]), width = WIDTH);

      last_line = Some(line_index);
    }
    Ok(())
  }

  fn done(&mut self) {}
}

fn collect_coverages(files: FileFlags) -> Result<Vec<cdp::ScriptCoverage>, AnyError> {
  let mut coverages: Vec<cdp::ScriptCoverage> = Vec::new();
  let file_paths = FileCollector::new(|file_path| file_path.extension().map(|ext| ext == "json").unwrap_or(false))
    .ignore_git_folder()
    .ignore_node_modules()
    .ignore_vendor_folder()
    .add_ignore_paths(&files.ignore)
    .collect_files(if files.include.is_empty() { None } else { Some(&files.include) })?;

  for file_path in file_paths {
    let json = fs::read_to_string(file_path.as_path())?;
    let new_coverage: cdp::ScriptCoverage = serde_json::from_str(&json)?;
    coverages.push(new_coverage);
  }

  coverages.sort_by_key(|k| k.url.clone());

  Ok(coverages)
}

fn filter_coverages(coverages: Vec<cdp::ScriptCoverage>, include: Vec<String>, exclude: Vec<String>, npm_resolver: &dyn CliNpmResolver) -> Vec<cdp::ScriptCoverage> {
  let include: Vec<Regex> = include.iter().map(|e| Regex::new(e).unwrap()).collect();

  let exclude: Vec<Regex> = exclude.iter().map(|e| Regex::new(e).unwrap()).collect();

  coverages
    .into_iter()
    .filter(|e| {
      let is_internal = e.url.starts_with("ext:")
        || e.url.ends_with("__anonymous__")
        || e.url.ends_with("$deno$test.js")
        || e.url.ends_with(".snap")
        || is_supported_test_path(Path::new(e.url.as_str()))
        || Url::parse(&e.url).ok().map(|url| npm_resolver.in_npm_package(&url)).unwrap_or(false);

      let is_included = include.iter().any(|p| p.is_match(&e.url));
      let is_excluded = exclude.iter().any(|p| p.is_match(&e.url));

      (include.is_empty() || is_included) && !is_excluded && !is_internal
    })
    .collect::<Vec<cdp::ScriptCoverage>>()
}

pub async fn cover_files(flags: Flags, coverage_flags: CoverageFlags) -> Result<(), AnyError> {
  if coverage_flags.files.include.is_empty() {
    return Err(generic_error("No matching coverage profiles found"));
  }

  let factory = CliFactory::from_flags(flags).await?;
  let npm_resolver = factory.npm_resolver().await?;
  let file_fetcher = factory.file_fetcher()?;
  let cli_options = factory.cli_options();
  let emitter = factory.emitter()?;

  let script_coverages = collect_coverages(coverage_flags.files)?;
  let script_coverages = filter_coverages(script_coverages, coverage_flags.include, coverage_flags.exclude, npm_resolver.as_ref());

  let proc_coverages: Vec<_> = script_coverages.into_iter().map(|cov| ProcessCoverage { result: vec![cov] }).collect();

  let script_coverages = if let Some(c) = merge::merge_processes(proc_coverages) { c.result } else { vec![] };

  let reporter_kind = if coverage_flags.lcov { CoverageReporterKind::Lcov } else { CoverageReporterKind::Pretty };

  let mut reporter = create_reporter(reporter_kind);

  let out_mode = match coverage_flags.output {
    Some(ref path) => match File::create(path) {
      Ok(_) => Some(PathBuf::from(path)),
      Err(e) => {
        return Err(anyhow!("Failed to create output file: {}", e));
      }
    },
    None => None,
  };

  for script_coverage in script_coverages {
    let module_specifier = deno_core::resolve_url_or_path(&script_coverage.url, cli_options.initial_cwd())?;

    let maybe_file = if module_specifier.scheme() == "file" {
      file_fetcher.get_source(&module_specifier)
    } else {
      file_fetcher.fetch_cached(&module_specifier, 10).with_context(|| format!("Failed to fetch \"{module_specifier}\" from cache."))?
    };
    let file = maybe_file.ok_or_else(|| {
      anyhow!(
        "Failed to fetch \"{}\" from cache.
          Before generating coverage report, run `deno test --coverage` to ensure consistent state.",
        module_specifier
      )
    })?;

    // Check if file was transpiled
    let original_source = file.source.clone();
    let transpiled_code: ModuleCode = match file.media_type {
      MediaType::JavaScript | MediaType::Unknown | MediaType::Cjs | MediaType::Mjs | MediaType::Json => file.source.clone().into(),
      MediaType::Dts | MediaType::Dmts | MediaType::Dcts => Default::default(),
      MediaType::TypeScript | MediaType::Jsx | MediaType::Mts | MediaType::Cts | MediaType::Tsx => match emitter.maybe_cached_emit(&file.specifier, &file.source) {
        Some(code) => code.into(),
        None => {
          return Err(anyhow!(
            "Missing transpiled source code for: \"{}\".
              Before generating coverage report, run `deno test --coverage` to ensure consistent state.",
            file.specifier,
          ))
        }
      },
      MediaType::Wasm | MediaType::TsBuildInfo | MediaType::SourceMap => {
        unreachable!()
      }
    };

    let source_map = source_map_from_code(&transpiled_code);
    let coverage_report = generate_coverage_report(&script_coverage, transpiled_code.as_str().to_owned(), &source_map, &out_mode);

    if !coverage_report.found_lines.is_empty() {
      reporter.report(&coverage_report, &original_source)?;
    }
  }

  reporter.done();

  Ok(())
}
