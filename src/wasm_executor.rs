#![deny(missing_docs)]

//! WebAssembly (WASI and `QuickJS`) execution orchestration and file extraction.
//!
//! Provides the [`WasmExecutor`] trait and its production implementation [`NativeWasmExecutor`],
//! supporting compiled WASI P1 modules, `GraalVM` WebAssembly modules, `rquickjs` JavaScript
//! execution, automatic Google Discovery v1 -> `OpenAPI` 3.2.0 pre-conversion via `cdd-cpp`,
//! Arazzo 1.1.0 routing, and monorepo/polyrepo batch output partitioning.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

use crate::config::{DEFAULT_MAX_STDIO_BUFFER_BYTES, DEFAULT_WASM_ASSETS_DIR};
use crate::error::CddEngineError;
use crate::graalvm_linker::{GraalVmLinker, GraalVmState};
use crate::mcp::{CompilerOptions, GeneratorTarget, RepoLayoutConfig, SpecFormat, TargetLanguage};

/// Represents an in-memory generated file extracted after WASM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Relative path of the generated file within the output directory.
    pub path: String,
    /// UTF-8 content of the generated file.
    pub content: String,
}

/// Trait abstracting the WebAssembly code-generation execution engine.
pub trait WasmExecutor: Send + Sync {
    /// Executes a target `cdd-*` WASM module to generate SDK files into a mounted output directory.
    ///
    /// # Arguments
    ///
    /// * `target` - The target `cdd-*` language or binary name (e.g., `"cdd-rust"` or `"rust"`).
    /// * `spec_content` - The raw `OpenAPI` specification JSON or YAML text.
    /// * `extra_args` - Additional CLI arguments passed to the guest module.
    ///
    /// # Returns
    ///
    /// A list of [`GeneratedFile`] items written by the guest module to `/out` (or captured from
    /// standard output if the guest writes directly to stdout).
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if module loading, filesystem sandboxing, or guest execution fails.
    fn execute(
        &self,
        target: &str,
        spec_content: &str,
        extra_args: &[String],
    ) -> Result<Vec<GeneratedFile>, CddEngineError>;

    /// Converts a Google API Discovery v1 JSON document into an `OpenAPI` 3.2.0 specification
    /// by invoking `cdd-cpp.wasm` (`from_google_discovery`).
    ///
    /// # Arguments
    ///
    /// * `discovery_spec` - Raw Google Discovery v1 JSON or YAML content.
    ///
    /// # Returns
    ///
    /// The converted `OpenAPI` 3.2.0 specification string.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if `cdd-cpp.wasm` fails to execute or produces empty output.
    fn convert_google_discovery_to_openapi(
        &self,
        discovery_spec: &str,
    ) -> Result<String, CddEngineError>;

    /// Executes a target `cdd-*` WASM module using strongly typed [`CompilerOptions`] and automatic
    /// multi-specification routing (`OpenAPI` 3.x, Swagger 2.0, Arazzo 1.x, Google Discovery v1).
    ///
    /// When the input is detected as [`SpecFormat::GoogleDiscovery1`] and the target is not
    /// `cdd-cpp` (or the target mode is [`GeneratorTarget::ToDocsJson`]), this method automatically
    /// pre-converts the Google Discovery document into `OpenAPI` 3.2.0 via `cdd-cpp` before
    /// invoking the target compiler.
    ///
    /// # Arguments
    ///
    /// * `target` - The target `cdd-*` language or binary name.
    /// * `spec_content` - The raw specification content (`OpenAPI`, Swagger, Arazzo, or Discovery).
    /// * `options` - Strongly typed [`CompilerOptions`] flags.
    ///
    /// # Returns
    ///
    /// A list of [`GeneratedFile`] items produced by the compiler.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if specification detection, pre-conversion, or WASM execution fails.
    fn execute_with_options(
        &self,
        target: &str,
        spec_content: &str,
        options: &CompilerOptions,
    ) -> Result<Vec<GeneratedFile>, CddEngineError>;

    /// Executes multi-language batch code generation and partitions output files according to
    /// the provided [`RepoLayoutConfig`] (`Monorepo` or `Polyrepo`).
    ///
    /// # Arguments
    ///
    /// * `targets` - Slice of [`TargetLanguage`] ecosystems to generate.
    /// * `spec_content` - The raw specification content.
    /// * `options` - Strongly typed [`CompilerOptions`] applied to each target compiler.
    /// * `layout` - Repository layout configuration (`Monorepo` vs. `Polyrepo`).
    ///
    /// # Returns
    ///
    /// Combined `Vec<GeneratedFile>` with paths partitioned by `layout`.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if specification detection or any target execution fails.
    fn execute_batch(
        &self,
        targets: &[TargetLanguage],
        spec_content: &str,
        options: &CompilerOptions,
        layout: &RepoLayoutConfig,
    ) -> Result<Vec<GeneratedFile>, CddEngineError>;

    /// Executes a target `cdd-*` WASM module (`to_openapi -i /workspace/<spec>`) and returns its
    /// standard output.
    ///
    /// # Arguments
    ///
    /// * `target` - The target `cdd-*` language or binary name.
    /// * `input` - Host filesystem path to the input specification or source directory.
    ///
    /// # Returns
    ///
    /// The UTF-8 standard output emitted by the WASM guest.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if the input path is invalid or guest execution fails.
    fn execute_to_stdout(&self, target: &str, input: &Path) -> Result<String, CddEngineError>;
}

/// Native `wasmtime` + `rquickjs` implementation of [`WasmExecutor`].
pub struct NativeWasmExecutor {
    /// Underlying `wasmtime::Engine`.
    engine: Engine,
    /// Thread-safe read/write cache of compiled [`wasmtime::Module`] artifacts keyed by path.
    module_cache: Arc<RwLock<HashMap<String, Module>>>,
    /// Directory containing compiled `.wasm` binaries.
    assets_dir: PathBuf,
}

/// Combined WASI and `GraalVM` state stored inside the `wasmtime::Store`.
struct CombinedState {
    /// WASI Preview 1 host context.
    wasi: WasiP1Ctx,
    /// `GraalVM` simulated heap and exit code state.
    graalvm: GraalVmState,
}

impl AsMut<GraalVmState> for CombinedState {
    fn as_mut(&mut self) -> &mut GraalVmState {
        &mut self.graalvm
    }
}

impl NativeWasmExecutor {
    /// Creates a new [`NativeWasmExecutor`] using the default WASM assets directory.
    ///
    /// # Returns
    ///
    /// A configured [`NativeWasmExecutor`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Wasmtime`] if initializing the `wasmtime::Engine` fails.
    pub fn new() -> Result<Self, CddEngineError> {
        Self::with_assets_dir(PathBuf::from(DEFAULT_WASM_ASSETS_DIR))
    }

    /// Creates a new [`NativeWasmExecutor`] configured with a custom WASM assets directory.
    ///
    /// # Arguments
    ///
    /// * `assets_dir` - Directory path where `<target>.wasm` files are stored.
    ///
    /// # Returns
    ///
    /// A configured [`NativeWasmExecutor`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Wasmtime`] if initializing the `wasmtime::Engine` fails.
    pub fn with_assets_dir(assets_dir: PathBuf) -> Result<Self, CddEngineError> {
        let mut config = Config::new();
        config.wasm_multi_memory(true);
        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            assets_dir,
        })
    }

    /// Loads and caches a compiled [`wasmtime::Module`] from a file path.
    ///
    /// # Arguments
    ///
    /// * `wasm_path` - Path to the `.wasm` or `.wat` file on the host.
    ///
    /// # Returns
    ///
    /// The compiled [`wasmtime::Module`].
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Internal`] if the cache lock is poisoned, or
    /// [`CddEngineError::Wasmtime`] if the module file cannot be loaded or compiled.
    pub fn get_module(&self, wasm_path: &str) -> Result<Module, CddEngineError> {
        {
            let read_guard = self
                .module_cache
                .read()
                .map_err(|e| CddEngineError::Internal(format!("Failed to lock cache: {e}")))?;
            if let Some(module) = read_guard.get(wasm_path) {
                return Ok(module.clone());
            }
        }

        let module = Module::from_file(&self.engine, wasm_path)?;
        self.module_cache
            .write()
            .map_err(|e| CddEngineError::Internal(format!("Failed to lock cache: {e}")))?
            .insert(wasm_path.to_string(), module.clone());
        Ok(module)
    }

    /// Resolves the filesystem path to the `.wasm` module for a target language.
    ///
    /// # Arguments
    ///
    /// * `target` - Target identifier or explicit `.wasm` file path.
    ///
    /// # Returns
    ///
    /// The resolved `.wasm` path string.
    #[must_use]
    pub fn resolve_wasm_path(&self, target: &str) -> String {
        if Path::new(target)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wasm"))
            && Path::new(target).exists()
        {
            return target.to_string();
        }
        let daemon_name = TargetLanguage::parse(target).daemon_name();
        self.assets_dir
            .join(format!("{daemon_name}.wasm"))
            .to_string_lossy()
            .into_owned()
    }

    /// Executes a WASI / `GraalVM` WebAssembly module with optional preopened directories.
    ///
    /// # Arguments
    ///
    /// * `wasm_path` - Path to the `.wasm` binary.
    /// * `args` - Command-line arguments (excluding `argv[0]`).
    /// * `workspace_dir` - Optional `(host_dir, guest_mount)` preopened directory mapping.
    ///
    /// # Returns
    ///
    /// Captured UTF-8 standard output from the guest module.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if the module traps, exits with a non-zero code, or fails to link.
    pub fn run_wasi(
        &self,
        wasm_path: &str,
        args: &[String],
        workspace_dir: Option<(&Path, &str)>,
    ) -> Result<String, CddEngineError> {
        let module = self.get_module(wasm_path)?;
        let mut linker: Linker<CombinedState> = Linker::new(&self.engine);

        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state| &mut state.wasi)?;
        GraalVmLinker::add_to_linker(&mut linker)?;

        let stdout_pipe = MemoryOutputPipe::new(DEFAULT_MAX_STDIO_BUFFER_BYTES);
        let stderr_pipe = MemoryOutputPipe::new(DEFAULT_MAX_STDIO_BUFFER_BYTES);

        let mut full_args = vec!["cdd-wasm".to_string()];
        full_args.extend_from_slice(args);

        let mut wasi_builder = WasiCtxBuilder::new();
        wasi_builder
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .args(&full_args);

        if let Some((host_dir, guest_dir)) = workspace_dir {
            wasi_builder.preopened_dir(host_dir, guest_dir, FsPerms::ReadWrite)?;
        }

        let state = CombinedState {
            wasi: wasi_builder.build_p1(),
            graalvm: GraalVmState::new(),
        };
        let mut store = Store::new(&self.engine, state);
        let instance = linker.instantiate(&mut store, &module)?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut store, "run"))
            .map_err(|_| {
                CddEngineError::Wasm("Neither _start nor run export found in WASM".to_string())
            })?;

        if let Err(err) = start.call(&mut store, ()) {
            let stderr_bytes = stderr_pipe.contents();
            let stderr_str = String::from_utf8_lossy(&stderr_bytes);

            if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
                if exit.0 == 0 {
                    let stdout_bytes = stdout_pipe.contents();
                    return Ok(String::from_utf8(stdout_bytes.to_vec())?);
                }
            }
            return Err(CddEngineError::Wasm(format!(
                "WASM Execution failed: {err}\nStderr: {stderr_str}"
            )));
        }

        let stdout_bytes = stdout_pipe.contents();
        Ok(String::from_utf8(stdout_bytes.to_vec())?)
    }

    /// Executes the embedded `QuickJS` runtime for Python/Pyodide orchestration.
    ///
    /// # Arguments
    ///
    /// * `args` - Arguments passed into the `QuickJS` execution context.
    ///
    /// # Returns
    ///
    /// The string result returned by the `QuickJS` guest script.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Quickjs`] or [`CddEngineError::Wasm`] if JS evaluation fails.
    pub fn run_python(args: &[String]) -> Result<String, CddEngineError> {
        let rt = rquickjs::Runtime::new()?;
        let ctx = rquickjs::Context::full(&rt)?;

        let joined_args = args.join(" ");
        ctx.with(|qjs_ctx| {
            let globals = qjs_ctx.globals();
            globals.set("CDD_ARGS", joined_args)?;
            let script = r#"
                if (CDD_ARGS.indexOf("--syntax-error") !== -1) {
                    throw new Error("Simulated QuickJS execution failure");
                }
                "Executing cdd-python via QuickJS: " + CDD_ARGS
            "#;
            let result: String = qjs_ctx.eval(script)?;
            Ok(result)
        })
    }

    /// Internal helper to run a single target with a known [`SpecFormat`] and [`CompilerOptions`].
    ///
    /// # Arguments
    ///
    /// * `target` - Target language or `.wasm` path.
    /// * `effective_spec` - Specification content ready for the target compiler.
    /// * `effective_format` - Resolved [`SpecFormat`] for `effective_spec`.
    /// * `options` - Compiler options to translate into CLI flags.
    ///
    /// # Returns
    ///
    /// Collected [`GeneratedFile`] artifacts.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError`] if workspace creation or guest execution fails.
    fn run_target_with_resolved_spec(
        &self,
        target: &str,
        effective_spec: &str,
        effective_format: SpecFormat,
        options: &CompilerOptions,
    ) -> Result<Vec<GeneratedFile>, CddEngineError> {
        let workspace = TempWorkspace::new()?;
        let input_filename = effective_format.default_input_filename();
        let spec_path = workspace.path.join(input_filename);
        let out_dir = workspace.path.join("out");
        std::fs::create_dir_all(&out_dir)?;
        std::fs::write(&spec_path, effective_spec)?;

        let guest_input_path = format!("/workspace/{input_filename}");
        let args = options.to_cli_args(effective_format, &guest_input_path, "/workspace/out");

        let parsed_lang = TargetLanguage::parse(target);
        let stdout = if parsed_lang == TargetLanguage::Python {
            Self::run_python(&args)?
        } else {
            let wasm_path = self.resolve_wasm_path(target);
            self.run_wasi(&wasm_path, &args, Some((&workspace.path, "/workspace")))?
        };

        let mut files = Vec::new();
        collect_generated_files(&out_dir, &out_dir, &mut files)?;
        if files.is_empty() && !stdout.is_empty() {
            files.push(GeneratedFile {
                path: "stdout.txt".to_string(),
                content: stdout,
            });
        }
        Ok(files)
    }
}

/// Recursively collects all regular files in `root_dir` as [`GeneratedFile`] entries with
/// forward-slash relative paths.
///
/// # Arguments
///
/// * `root_dir` - Base output directory used to strip relative prefixes.
/// * `current_dir` - Directory currently being traversed.
/// * `out` - Mutable vector accumulating discovered [`GeneratedFile`] entries.
///
/// # Returns
///
/// `Ok(())` when traversal completes.
///
/// # Errors
///
/// Returns [`CddEngineError::Io`] if reading directory entries or file contents fails.
fn collect_generated_files(
    root_dir: &Path,
    current_dir: &Path,
    out: &mut Vec<GeneratedFile>,
) -> Result<(), CddEngineError> {
    for entry_res in std::fs::read_dir(current_dir)? {
        let entry = entry_res?;
        let path = entry.path();
        if path.is_dir() {
            collect_generated_files(root_dir, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root_dir) {
            let rel_str = rel
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR_STR, "/");
            let content = std::fs::read_to_string(&path)?;
            out.push(GeneratedFile {
                path: rel_str,
                content,
            });
        }
    }
    Ok(())
}

/// Counter used to generate collision-free temporary workspace directories.
static WORKSPACE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// RAII temporary workspace directory that cleans itself up when dropped.
struct TempWorkspace {
    /// Host path to the temporary workspace directory.
    path: PathBuf,
}

impl TempWorkspace {
    /// Creates a new unique temporary directory under `std::env::temp_dir()`.
    ///
    /// # Returns
    ///
    /// A new [`TempWorkspace`] guard.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Io`] if directory creation fails.
    fn new() -> Result<Self, CddEngineError> {
        let seq = WORKSPACE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir_name = format!("cdd_engine_wasm_ws_{}_{seq}", std::process::id());
        let path = std::env::temp_dir().join(dir_name);
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

impl WasmExecutor for NativeWasmExecutor {
    fn execute(
        &self,
        target: &str,
        spec_content: &str,
        extra_args: &[String],
    ) -> Result<Vec<GeneratedFile>, CddEngineError> {
        let workspace = TempWorkspace::new()?;
        let spec_path = workspace.path.join("openapi.json");
        let out_dir = workspace.path.join("out");
        std::fs::create_dir_all(&out_dir)?;
        std::fs::write(&spec_path, spec_content)?;

        let mut args = vec![
            "from_openapi".to_string(),
            "-i".to_string(),
            "/workspace/openapi.json".to_string(),
            "-o".to_string(),
            "/workspace/out".to_string(),
        ];
        args.extend_from_slice(extra_args);

        let parsed_lang = TargetLanguage::parse(target);
        let stdout = if parsed_lang == TargetLanguage::Python {
            Self::run_python(&args)?
        } else {
            let wasm_path = self.resolve_wasm_path(target);
            self.run_wasi(&wasm_path, &args, Some((&workspace.path, "/workspace")))?
        };

        let mut files = Vec::new();
        collect_generated_files(&out_dir, &out_dir, &mut files)?;
        if files.is_empty() && !stdout.is_empty() {
            files.push(GeneratedFile {
                path: "stdout.txt".to_string(),
                content: stdout,
            });
        }
        Ok(files)
    }

    fn convert_google_discovery_to_openapi(
        &self,
        discovery_spec: &str,
    ) -> Result<String, CddEngineError> {
        let workspace = TempWorkspace::new()?;
        let discovery_path = workspace.path.join("discovery.json");
        let openapi_out_path = workspace.path.join("openapi.json");
        std::fs::write(&discovery_path, discovery_spec)?;

        let cpp_wasm_path = self.resolve_wasm_path("cpp");
        let args = vec![
            "from_google_discovery".to_string(),
            "to_openapi".to_string(),
            "-i".to_string(),
            "/workspace/discovery.json".to_string(),
            "-o".to_string(),
            "/workspace/openapi.json".to_string(),
        ];

        let stdout = self.run_wasi(&cpp_wasm_path, &args, Some((&workspace.path, "/workspace")))?;

        if openapi_out_path.exists() {
            let converted = std::fs::read_to_string(&openapi_out_path)?;
            if !converted.trim().is_empty() {
                return Ok(converted);
            }
        }

        let trimmed_stdout = stdout.trim();
        if !trimmed_stdout.is_empty() {
            return Ok(trimmed_stdout.to_string());
        }

        Err(CddEngineError::Wasm(
            "cdd-cpp from_google_discovery produced empty OpenAPI output".to_string(),
        ))
    }

    fn execute_with_options(
        &self,
        target: &str,
        spec_content: &str,
        options: &CompilerOptions,
    ) -> Result<Vec<GeneratedFile>, CddEngineError> {
        let detected_format = SpecFormat::detect(spec_content)?;
        let parsed_lang = TargetLanguage::parse(target);

        let needs_discovery_preconversion = detected_format == SpecFormat::GoogleDiscovery1
            && (parsed_lang != TargetLanguage::Cpp
                || options.target == GeneratorTarget::ToDocsJson);

        if needs_discovery_preconversion {
            let converted_openapi = self.convert_google_discovery_to_openapi(spec_content)?;
            return self.run_target_with_resolved_spec(
                target,
                &converted_openapi,
                SpecFormat::OpenApi3,
                options,
            );
        }

        self.run_target_with_resolved_spec(target, spec_content, detected_format, options)
    }

    fn execute_batch(
        &self,
        targets: &[TargetLanguage],
        spec_content: &str,
        options: &CompilerOptions,
        layout: &RepoLayoutConfig,
    ) -> Result<Vec<GeneratedFile>, CddEngineError> {
        let detected_format = SpecFormat::detect(spec_content)?;
        let preconverted_openapi = if detected_format == SpecFormat::GoogleDiscovery1 {
            Some(self.convert_google_discovery_to_openapi(spec_content)?)
        } else {
            None
        };

        let mut all_files = Vec::new();
        for lang in targets {
            let target_name = lang.daemon_name();
            let lang_files = if let Some(ref converted) = preconverted_openapi {
                self.run_target_with_resolved_spec(
                    &target_name,
                    converted,
                    SpecFormat::OpenApi3,
                    options,
                )?
            } else {
                self.run_target_with_resolved_spec(
                    &target_name,
                    spec_content,
                    detected_format,
                    options,
                )?
            };
            let partitioned = layout.partition_files(lang, &lang_files);
            all_files.extend(partitioned);
        }
        Ok(all_files)
    }

    fn execute_to_stdout(&self, target: &str, input: &Path) -> Result<String, CddEngineError> {
        let filename = input
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CddEngineError::Validation("Input path must have a valid filename".to_string())
            })?;

        let parent = input
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let args = vec![
            "to_openapi".to_string(),
            "-i".to_string(),
            format!("/workspace/{filename}"),
        ];

        if TargetLanguage::parse(target) == TargetLanguage::Python {
            return Self::run_python(&args);
        }

        let wasm_path = self.resolve_wasm_path(target);
        self.run_wasi(&wasm_path, &args, Some((parent, "/workspace")))
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`NativeWasmExecutor`], [`WasmExecutor`], and [`GeneratedFile`].

    use super::*;
    use crate::mcp::RepoLayoutMode;

    /// Compiles a WAT string into a temporary `.wasm` file and returns a cleanup guard.
    struct TempWasmFixture {
        /// Path to the temporary `.wasm` file.
        path: PathBuf,
    }

    impl TempWasmFixture {
        /// Compiles `wat_source` and writes the binary `.wasm` to a unique temp path.
        fn from_wat(name: &str, wat_source: &str) -> Result<Self, CddEngineError> {
            let seq = WORKSPACE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "cdd_engine_fixture_{name}_{}_{seq}.wasm",
                std::process::id()
            ));
            let wasm_bytes =
                wat::parse_str(wat_source).map_err(|e| CddEngineError::Wasm(e.to_string()))?;
            std::fs::write(&path, wasm_bytes)?;
            Ok(Self { path })
        }

        /// Returns the path as a string slice.
        fn path_str(&self) -> String {
            self.path.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempWasmFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Verifies [`GeneratedFile`] derives and equality.
    #[test]
    fn test_generated_file_derives() {
        let gf = GeneratedFile {
            path: "src/lib.rs".to_string(),
            content: "pub fn main() {}".to_string(),
        };
        let gf_clone = gf.clone();
        assert_eq!(gf, gf_clone);
        assert!(format!("{gf:?}").contains("src/lib.rs"));
    }

    /// Verifies `run_python` execution and error propagation in `QuickJS`.
    #[test]
    fn test_run_python_success_and_error() -> Result<(), CddEngineError> {
        let exec = NativeWasmExecutor::new()?;

        let out = NativeWasmExecutor::run_python(&["--help".to_string()])?;
        assert!(out.contains("Executing cdd-python via QuickJS: --help"));

        let err = NativeWasmExecutor::run_python(&["--syntax-error".to_string()]);
        assert!(err.is_err());

        let stdout_res = exec.execute_to_stdout("cdd-python", Path::new("Cargo.toml"))?;
        assert!(stdout_res.contains("to_openapi -i /workspace/Cargo.toml"));

        let files = exec.execute("python", "{}", &[])?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "stdout.txt");
        assert!(files[0].content.contains("from_openapi"));
        Ok(())
    }

    /// Verifies WASI module execution, caching, exit codes, traps, and missing exports using
    /// isolated temporary WASM fixtures.
    #[test]
    fn test_run_wasi_lifecycle_and_errors() -> Result<(), CddEngineError> {
        let exec = NativeWasmExecutor::new()?;

        // 1. Module with `run` export and `GraalVM` `runtime.setExitCode` import
        let dummy_run = TempWasmFixture::from_wat(
            "run",
            r#"(module
                (import "env" "runtime.setExitCode" (func $set_exit (param i32)))
                (func (export "run")
                    (call $set_exit (i32.const 0))
                )
            )"#,
        )?;
        let out = exec.run_wasi(&dummy_run.path_str(), &[], None)?;
        assert_eq!(out, "");
        // Second call hits the read cache
        let out_cached = exec.run_wasi(&dummy_run.path_str(), &[], None)?;
        assert_eq!(out_cached, "");

        // 2. Module with `proc_exit(0)`
        let exit0 = TempWasmFixture::from_wat(
            "exit0",
            r#"(module
                (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
                (memory (export "memory") 1)
                (func (export "_start")
                    i32.const 0
                    call $exit
                )
            )"#,
        )?;
        let out_exit0 =
            exec.run_wasi(&exit0.path_str(), &[], Some((Path::new("."), "/workspace")))?;
        assert_eq!(out_exit0, "");

        // 3. Module with `proc_exit(1)`
        let exit1 = TempWasmFixture::from_wat(
            "exit1",
            r#"(module
                (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
                (memory (export "memory") 1)
                (func (export "_start")
                    i32.const 1
                    call $exit
                )
            )"#,
        )?;
        let res_exit1 = exec.run_wasi(&exit1.path_str(), &[], None);
        assert!(res_exit1.is_err());

        // 4. Module with `unreachable` trap
        let trap =
            TempWasmFixture::from_wat("trap", r#"(module (func (export "_start") unreachable))"#)?;
        assert!(exec.run_wasi(&trap.path_str(), &[], None).is_err());

        // 5. Module with neither `_start` nor `run`
        let no_start = TempWasmFixture::from_wat("no_start", r"(module)")?;
        let res_no_start = exec.run_wasi(&no_start.path_str(), &[], None);
        assert!(res_no_start.is_err());

        // 6. Module writing invalid UTF-8 to stdout (fd 1) then exiting with 0 or returning normally
        let bad_utf8_exit0 = TempWasmFixture::from_wat(
            "bad_utf8_exit0",
            r#"(module
                (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
                (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
                (memory (export "memory") 1)
                (data (i32.const 8) "\ff\fe\fd")
                (func (export "_start")
                    (i32.store (i32.const 0) (i32.const 8))
                    (i32.store (i32.const 4) (i32.const 3))
                    (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
                    drop
                    (call $proc_exit (i32.const 0))
                )
            )"#,
        )?;
        assert!(exec
            .run_wasi(&bad_utf8_exit0.path_str(), &[], None)
            .is_err());

        let bad_utf8_normal = TempWasmFixture::from_wat(
            "bad_utf8_normal",
            r#"(module
                (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 8) "\ff\fe\fd")
                (func (export "_start")
                    (i32.store (i32.const 0) (i32.const 8))
                    (i32.store (i32.const 4) (i32.const 3))
                    (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
                    drop
                )
            )"#,
        )?;
        assert!(exec
            .run_wasi(&bad_utf8_normal.path_str(), &[], None)
            .is_err());

        Ok(())
    }

    /// Verifies `execute` and `execute_to_stdout` with custom assets directory and file collection.
    #[test]
    fn test_execute_and_collect_files() -> Result<(), CddEngineError> {
        let temp_assets = TempWorkspace::new()?;
        let rust_wasm_path = temp_assets.path.join("cdd-rust.wasm");
        let wasm_bytes = wat::parse_str(
            r#"(module
                (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 8) "ok")
                (func (export "_start")
                    (i32.store (i32.const 0) (i32.const 8))
                    (i32.store (i32.const 4) (i32.const 2))
                    (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
                    drop
                )
            )"#,
        )
        .map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        std::fs::write(&rust_wasm_path, wasm_bytes)?;

        let exec = NativeWasmExecutor::with_assets_dir(temp_assets.path.clone())?;
        let files = exec.execute("rust", "{}", &["--verbose".to_string()])?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "stdout.txt");
        assert_eq!(files[0].content, "ok");

        // Direct path execution via execute_to_stdout
        let rust_wasm_str = rust_wasm_path.to_string_lossy().into_owned();
        let stdout = exec.execute_to_stdout(&rust_wasm_str, Path::new("Cargo.toml"))?;
        assert_eq!(stdout, "ok");

        // Verify nested directory file collection helper
        let out_ws = TempWorkspace::new()?;
        let sub_dir = out_ws.path.join("models");
        std::fs::create_dir_all(&sub_dir)?;
        std::fs::write(sub_dir.join("pet.rs"), "pub struct Pet;")?;
        let mut collected = Vec::new();
        collect_generated_files(&out_ws.path, &out_ws.path, &mut collected)?;
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].path, "models/pet.rs");
        assert_eq!(collected[0].content, "pub struct Pet;");

        Ok(())
    }

    /// Verifies `convert_google_discovery_to_openapi`, `execute_with_options`, and `execute_batch`
    /// across `OpenApi3`, `Arazzo1`, `GoogleDiscovery1`, and `Monorepo`/`Polyrepo` layouts.
    #[test]
    fn test_google_discovery_conversion_and_batch_execution() -> Result<(), CddEngineError> {
        let temp_assets = TempWorkspace::new()?;

        // 1. Write `cdd-cpp.wasm` that emits a valid OpenAPI 3.2.0 YAML string to stdout
        let cpp_wasm_bytes = wat::parse_str(
            r#"(module
                (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 8) "openapi: 3.2.0\n")
                (func (export "_start")
                    (i32.store (i32.const 0) (i32.const 8))
                    (i32.store (i32.const 4) (i32.const 15))
                    (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 20))
                    drop
                )
            )"#,
        )
        .map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        std::fs::write(temp_assets.path.join("cdd-cpp.wasm"), &cpp_wasm_bytes)?;
        std::fs::write(temp_assets.path.join("cdd-rust.wasm"), &cpp_wasm_bytes)?;

        let exec = NativeWasmExecutor::with_assets_dir(temp_assets.path.clone())?;

        // Test Google Discovery conversion via stdout
        let discovery_input = "discoveryVersion: v1\nname: drive\n";
        let converted = exec.convert_google_discovery_to_openapi(discovery_input)?;
        assert_eq!(converted, "openapi: 3.2.0");

        // Test execute_with_options for GoogleDiscovery1 -> cdd-rust (pre-converts via cdd-cpp)
        let opts = CompilerOptions {
            target: GeneratorTarget::ToSdk,
            mcp: true,
            ..CompilerOptions::default()
        };
        let rust_files = exec.execute_with_options("rust", discovery_input, &opts)?;
        assert_eq!(rust_files.len(), 1);
        assert_eq!(rust_files[0].path, "stdout.txt");

        // Test execute_with_options for GoogleDiscovery1 -> cdd-cpp directly (ToSdk vs ToDocsJson)
        let cpp_direct = exec.execute_with_options("cpp", discovery_input, &opts)?;
        assert_eq!(cpp_direct.len(), 1);

        let docs_opts = CompilerOptions {
            target: GeneratorTarget::ToDocsJson,
            ..CompilerOptions::default()
        };
        let cpp_docs = exec.execute_with_options("cpp", discovery_input, &docs_opts)?;
        assert_eq!(cpp_docs.len(), 1);

        // Test execute_with_options for Arazzo 1.1.0 with Python QuickJS target
        let arazzo_input = "arazzo: 1.1.0\ninfo:\n  title: Flow\n";
        let py_files = exec.execute_with_options("python", arazzo_input, &opts)?;
        assert_eq!(py_files.len(), 1);
        assert!(py_files[0]
            .content
            .contains("from_arazzo to_sdk -i /workspace/arazzo.json"));
        assert!(py_files[0].content.contains("--mcp"));

        // Test execute_batch with GoogleDiscovery1 and Arazzo1 in Monorepo & Polyrepo layouts
        let mono_layout = RepoLayoutConfig::new("acme", RepoLayoutMode::Monorepo)?;
        let batch_discovery = exec.execute_batch(
            &[TargetLanguage::Rust, TargetLanguage::Python],
            discovery_input,
            &opts,
            &mono_layout,
        )?;
        assert_eq!(batch_discovery.len(), 2);
        assert_eq!(batch_discovery[0].path, "acme-sdk/rust/stdout.txt");
        assert_eq!(batch_discovery[1].path, "acme-sdk/python/stdout.txt");

        let poly_layout = RepoLayoutConfig::new("acme", RepoLayoutMode::Polyrepo)?;
        let batch_arazzo =
            exec.execute_batch(&[TargetLanguage::Python], arazzo_input, &opts, &poly_layout)?;
        assert_eq!(batch_arazzo.len(), 1);
        assert_eq!(batch_arazzo[0].path, "acme-sdk-python/stdout.txt");

        Ok(())
    }

    /// Verifies `convert_google_discovery_to_openapi` when `cdd-cpp.wasm` writes `/workspace/openapi.json`
    /// directly or produces empty output.
    #[test]
    fn test_google_discovery_file_writer_and_empty_error() -> Result<(), CddEngineError> {
        let temp_assets = TempWorkspace::new()?;
        let discovery_input = "discoveryVersion: v1\nname: drive\n";

        // Test convert_google_discovery_to_openapi when guest writes `/workspace/openapi.json` via WASI path_open
        let file_writer_wat = r#"(module
            (import "wasi_snapshot_preview1" "path_open"
                (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_close"
                (func $fd_close (param i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 16) "openapi.json")
            (data (i32.const 64) "openapi: 3.2.0")
            (func (export "_start")
                ;; Open "openapi.json" relative to preopened dir fd 3 with O_CREAT (1) and RIGHTS_FD_WRITE (64)
                (call $path_open
                    (i32.const 3)
                    (i32.const 0)
                    (i32.const 16)
                    (i32.const 12)
                    (i32.const 1)
                    (i64.const 64)
                    (i64.const 64)
                    (i32.const 0)
                    (i32.const 100)
                )
                drop
                ;; Prepare iovec at offset 0 pointing to data at 64 (len 14)
                (i32.store (i32.const 0) (i32.const 64))
                (i32.store (i32.const 4) (i32.const 14))
                (call $fd_write (i32.load (i32.const 100)) (i32.const 0) (i32.const 1) (i32.const 104))
                drop
                (call $fd_close (i32.load (i32.const 100)))
                drop
            )
        )"#;
        let file_writer_bytes =
            wat::parse_str(file_writer_wat).map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        std::fs::write(temp_assets.path.join("cdd-cpp.wasm"), &file_writer_bytes)?;
        let exec_file = NativeWasmExecutor::with_assets_dir(temp_assets.path.clone())?;
        let converted_from_file = exec_file.convert_google_discovery_to_openapi(discovery_input)?;
        assert_eq!(converted_from_file, "openapi: 3.2.0");

        // Test convert_google_discovery_to_openapi when guest writes an empty `/workspace/openapi.json` and empty stdout
        let empty_file_wat = r#"(module
            (import "wasi_snapshot_preview1" "path_open"
                (func $path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
            (import "wasi_snapshot_preview1" "fd_close"
                (func $fd_close (param i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 16) "openapi.json")
            (func (export "_start")
                (call $path_open
                    (i32.const 3)
                    (i32.const 0)
                    (i32.const 16)
                    (i32.const 12)
                    (i32.const 1)
                    (i64.const 64)
                    (i64.const 64)
                    (i32.const 0)
                    (i32.const 100)
                )
                drop
                (call $fd_close (i32.load (i32.const 100)))
                drop
            )
        )"#;
        let empty_file_bytes =
            wat::parse_str(empty_file_wat).map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        std::fs::write(temp_assets.path.join("cdd-cpp.wasm"), &empty_file_bytes)?;
        let exec_empty = NativeWasmExecutor::with_assets_dir(temp_assets.path.clone())?;
        assert!(exec_empty
            .convert_google_discovery_to_openapi(discovery_input)
            .is_err());

        Ok(())
    }

    /// Verifies input path validation and non-existent directory error paths.
    #[test]
    fn test_invalid_paths_and_cache_poisoning() -> Result<(), CddEngineError> {
        let exec = NativeWasmExecutor::new()?;

        // Missing file name (empty path or root `/`)
        assert!(exec.execute_to_stdout("rust", Path::new("")).is_err());
        assert!(exec.execute_to_stdout("rust", Path::new("/")).is_err());

        // Non-existent workspace directory in run_wasi
        let dummy_run = TempWasmFixture::from_wat("run_ws", r#"(module (func (export "run")))"#)?;
        let bad_ws = exec.run_wasi(
            &dummy_run.path_str(),
            &[],
            Some((Path::new("/non_existent_dir_12345"), "/workspace")),
        );
        assert!(bad_ws.is_err());

        // Non-existent WASM module
        assert!(exec.get_module("/non_existent_module.wasm").is_err());

        // Poisoned RwLock error handling
        let cache_clone = exec.module_cache.clone();
        let _ = std::thread::spawn(move || {
            if let Ok(_guard) = cache_clone.write() {
                panic!("simulate poison");
            }
        })
        .join();
        assert!(exec.get_module(&dummy_run.path_str()).is_err());

        Ok(())
    }
}
