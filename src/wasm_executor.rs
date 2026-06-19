use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

/// A generated file output from a WASM execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratedFile {
    /// The relative path of the file.
    pub path: String,
    /// The raw byte content of the file.
    pub content: Vec<u8>,
}

/// A standard trait for executing SDK generators.
pub trait WasmExecutor: Send + Sync {
    /// Executes the target returning a list of generated files from the `/out` directory.
    fn execute(
        &self,
        target: &str,
        input: &str,
        args: &[String],
    ) -> Result<Vec<GeneratedFile>, crate::error::CddEngineError>;
    /// Executes the target returning the raw stdout bytes (typically for JSON outputs).
    fn execute_to_stdout(
        &self,
        target: &str,
        input: &str,
        args: &[String],
    ) -> Result<Vec<u8>, crate::error::CddEngineError>;
    /// Executes a raw WASI command for the CLI, returning (stdout, stderr).
    fn execute_cli(
        &self,
        target: &str,
        input_dir: Option<&Path>,
        mount_current_dir: bool,
        args: &[String],
    ) -> Result<(Vec<u8>, Vec<u8>), crate::error::CddEngineError>;
}

/// Native implementation using the `wasmtime` embedded engine.
pub struct NativeWasmExecutor {
    /// The Wasmtime engine.
    engine: Engine,
    /// A cache of loaded WASM modules.
    module_cache: Arc<Mutex<HashMap<String, Module>>>,
}

/// A globally shared instance of the native WASM executor.
pub static WASM_EXECUTOR: Lazy<NativeWasmExecutor> =
    Lazy::new(|| NativeWasmExecutor::new().expect("Failed to initialize WASM engine"));

impl NativeWasmExecutor {
    /// Initializes a new embedded `wasmtime` engine.
    pub fn new() -> Result<Self, String> {
        let mut config = Config::new();
        config.wasm_gc(true);
        config.wasm_function_references(true);
        config.wasm_multi_memory(true);
        config.wasm_memory64(true);

        let engine = Engine::new(&config).expect("Engine error");

        Ok(Self {
            engine,
            module_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Runs a python script using Pyodide via embedded QuickJS.
    fn run_python(
        &self,
        target: &str,
        _input_dir: Option<&Path>,
        _args: &[String],
    ) -> Result<(Vec<u8>, Vec<u8>), crate::error::CddEngineError> {
        // Here we will embed QuickJS to orchestrate the Pyodide WebAssembly module.
        use rquickjs::{Context, Runtime};

        let rt = Runtime::new().expect("QuickJS runtime init failed");
        let _ctx = Context::full(&rt).expect("QuickJS context init failed");

        // This is a stub implementation representing Phase 2 of PLANNN.md
        // To be fully implemented with pyodide.mjs injection.
        let stdout = format!(
            "Executing {} via Pyodide logic inside rquickjs (STUB)",
            target
        )
        .into_bytes();
        let stderr = vec![];

        Ok((stdout, stderr))
    }

    /// Retrieves a module from the cache or loads it from disk.
    fn get_module(&self, wasm_file: &str) -> Result<Module, crate::error::CddEngineError> {
        let mut cache = self.module_cache.lock().expect("Mutex lock failed");
        if let Some(module) = cache.get(wasm_file) {
            return Ok(module.clone());
        }

        let module = Module::from_file(&self.engine, wasm_file).map_err(|e| {
            crate::error::CddEngineError::Wasm(format!("Failed to load {}: {}", wasm_file, e))
        })?;

        cache.insert(wasm_file.to_string(), module.clone());
        Ok(module)
    }

    /// Runs a WASI module with the given arguments and environment.
    fn run_wasi(
        &self,
        target: &str,
        input_dir: Option<&Path>,
        mount_current_dir: bool,
        args: &[String],
        wasm_file_override: Option<&str>,
    ) -> Result<(Vec<u8>, Vec<u8>), crate::error::CddEngineError> {
        let wasm_file = wasm_file_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("cdd-ctl-wasm-sdk/assets/wasm/{}.wasm", target));
        let module = self.get_module(&wasm_file)?;

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |ctx| ctx).expect("Failed to link WASI");

        let stdout = MemoryOutputPipe::new(1024 * 1024 * 10); // 10MB
        let stderr = MemoryOutputPipe::new(1024 * 1024 * 10);

        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone()).stderr(stderr.clone());

        if let Some(dir) = input_dir {
            builder.preopened_dir(dir, "/workspace", DirPerms::all(), FilePerms::all())?;
        }
        if mount_current_dir {
            builder
                .preopened_dir(".", ".", DirPerms::all(), FilePerms::all())
                .expect("err");
        }

        let mut wasi_args = vec![wasm_file.clone()];
        wasi_args.extend(args.iter().cloned());
        builder.args(&wasi_args);

        let ctx = builder.build_p1();
        let mut store = Store::new(&self.engine, ctx);

        let instance = linker.instantiate(&mut store, &module)?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| {
                crate::error::CddEngineError::Wasm(format!("No _start function: {}", e))
            })?;

        if let Err(err) = start.call(&mut store, ()) {
            let is_success_exit = if let Some(exit) = err.downcast_ref::<wasmtime_wasi::I32Exit>() {
                exit.0 == 0
            } else {
                false
            };

            if !is_success_exit {
                let stderr_bytes = stderr.contents();
                let stderr_str = String::from_utf8_lossy(&stderr_bytes);
                return Err(crate::error::CddEngineError::Wasm(format!(
                    "Execution failed: {}\nStderr: {}",
                    err, stderr_str
                )));
            }
        }

        Ok((stdout.contents().into(), stderr.contents().into()))
    }
}

impl WasmExecutor for NativeWasmExecutor {
    fn execute(
        &self,
        _target: &str,
        _input: &str,
        _args: &[String],
    ) -> Result<Vec<GeneratedFile>, crate::error::CddEngineError> {
        Ok(vec![])
    }

    fn execute_to_stdout(
        &self,
        target: &str,
        input: &str,
        args: &[String],
    ) -> Result<Vec<u8>, crate::error::CddEngineError> {
        let input_path = std::path::Path::new(input)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(input));
        let input_dir = input_path.parent();
        let filename = input_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut run_args = vec![];
        for a in args {
            run_args.push(a.clone());
        }
        run_args.push("-i".to_string());
        run_args.push(format!("/workspace/{}", filename));

        let (stdout, _) = if target == "cdd-python" || target == "cdd-python-all" {
            self.run_python(target, input_dir, &run_args)
                .expect("run python err")
        } else if target == "cdd-sh" {
            let mut sh_args = vec!["/workspace/script.sh".to_string()];
            sh_args.extend(run_args);
            self.run_wasi(
                "dash",
                input_dir,
                false,
                &sh_args,
                Some("cdd-ctl-wasm-sdk/assets/wasm/dash.wasm"),
            )?
        } else {
            self.run_wasi(target, input_dir, false, &run_args, None)?
        };
        Ok(stdout)
    }

    fn execute_cli(
        &self,
        target: &str,
        input_dir: Option<&Path>,
        mount_current_dir: bool,
        args: &[String],
    ) -> Result<(Vec<u8>, Vec<u8>), crate::error::CddEngineError> {
        if target == "cdd-python" || target == "cdd-python-all" {
            Ok(self
                .run_python(target, input_dir, args)
                .expect("run python err"))
        } else if target == "cdd-sh" {
            let mut sh_args = vec!["/workspace/script.sh".to_string()];
            sh_args.extend(args.iter().cloned());
            self.run_wasi(
                "dash",
                input_dir,
                mount_current_dir,
                &sh_args,
                Some("cdd-ctl-wasm-sdk/assets/wasm/dash.wasm"),
            )
        } else {
            self.run_wasi(target, input_dir, mount_current_dir, args, None)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_executor() {
        let exec = NativeWasmExecutor::new().expect("wasm execute test error");

        // test get_module caches
        let wasm_file = "dummy_wasi.wasm";
        let _m1 = exec.get_module(wasm_file).expect("wasm execute test error");
        let _m2 = exec.get_module(wasm_file).expect("wasm execute test error"); // from cache

        let (stdout, _stderr) = exec
            .run_wasi(
                "target_does_not_matter",
                None,
                false,
                &["arg1".to_string()],
                Some(wasm_file),
            )
            .expect("wasm execute test error");
        let stdout_str = String::from_utf8_lossy(&stdout);
        assert!(stdout_str.contains("Hello from WASI"));

        // error load
        assert!(exec.get_module("does_not_exist.wasm").is_err());

        // execute
        let res = exec.execute("test", "test", &[]);
        assert!(res.expect("wasm execute test error").is_empty());

        // execute_to_stdout dummy
        let stdout = exec
            .execute_to_stdout("cdd-python", "test", &[])
            .expect("wasm execute test error");
        let s = String::from_utf8_lossy(&stdout);
        assert!(s.contains("Executing cdd-python via Pyodide logic inside rquickjs (STUB)"));

        let _stdout = exec
            .execute_to_stdout("cdd-sh", "test", &[])
            .expect_err("wasm execute expected to fail"); // will fail to load dash.wasm

        let _stdout = exec
            .execute_to_stdout("other", "test", &[])
            .expect_err("wasm execute expected to fail"); // will fail to load other.wasm

        // execute_cli
        let res = exec
            .execute_cli("cdd-python", None, false, &[])
            .expect("wasm execute test error");
        assert!(String::from_utf8_lossy(&res.0).contains("Executing cdd-python via Pyodide logic"));

        let _res = exec
            .execute_cli("cdd-sh", None, false, &[])
            .expect_err("wasm execute expected to fail"); // dash.wasm missing

        let _res = exec
            .execute_cli("other", None, true, &[])
            .expect_err("wasm execute expected to fail"); // other.wasm missing

        // run_wasi error
        assert!(exec
            .run_wasi("target", None, false, &[], Some("does_not_exist.wasm"))
            .is_err());

        // mount current dir
        let (stdout, _) = exec
            .run_wasi("target", None, true, &[], Some("dummy_wasi.wasm"))
            .expect("wasm execute test error");
        assert!(String::from_utf8_lossy(&stdout).contains("Hello"));
    }

    #[test]
    fn test_wasm_executor_coverage_cases() {
        let exec = &WASM_EXECUTOR; // covers line 75 Lazy init

        // cover args loop in python
        let python_res = exec
            .execute_to_stdout("cdd-python", "print(\"test\")", &["--debug".to_string()])
            .expect("wasm execute test error");
        assert!(
            String::from_utf8_lossy(&python_res).contains("Executing cdd-python via Pyodide logic")
        );

        // cover input_dir error
        let bad_dir = exec.run_wasi(
            "target",
            Some(std::path::Path::new("invalid_dir_that_doesnt_exist_1234")),
            true,
            &[],
            Some("dummy_wasi.wasm"),
        );
        assert!(bad_dir.is_err());

        // cover missing _start
        let missing_start = exec.run_wasi("target", None, false, &[], Some("dummy_lib.wasm"));
        assert!(missing_start.is_err());

        // cover exit 1
        let exit_1 = exec.run_wasi("target", None, false, &[], Some("dummy_fail_wasi.wasm"));
        assert!(exit_1.is_err());

        // cover exit 0 explicitly
        let exit_0 = exec.run_wasi("target", None, false, &[], Some("dummy_exit0.wasm"));
        assert!(exit_0.is_ok());

        // cover instantiate failure
        let instantiate_fail =
            exec.run_wasi("target", None, false, &[], Some("dummy_bad_import.wasm"));
        assert!(instantiate_fail.is_err());

        // cover trap (non I32Exit error)
        let trap = exec.run_wasi("target", None, false, &[], Some("dummy_trap.wasm"));
        assert!(trap.is_err());
    }
}
