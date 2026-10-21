#![deny(missing_docs)]

//! `GraalVM` WebAssembly host import bindings and virtual heap state.
//!
//! Provides [`GraalVmState`] for tracking guest JS-interop objects,stdout/stderr streams,
//! and exit codes, and [`GraalVmLinker`] for wiring `wasmtime::Linker` imports expected by
//! `GraalVM` native-image WebAssembly modules.

use crate::error::CddEngineError;
use std::collections::HashMap;
use wasmtime::{Caller, Linker, Memory};

/// State object holding simulated JS heap references and captured I/O for `GraalVM` guests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraalVmState {
    /// Simulated JS heap for interop.
    pub heap: HashMap<i32, String>,
    /// Next available object identifier on the simulated heap.
    pub next_id: i32,
    /// Captured standard output characters emitted via `stdoutWriter.printChars`.
    pub stdout_buf: String,
    /// Captured standard error characters emitted via `stderrWriter.printChars`.
    pub stderr_buf: String,
    /// Optional guest exit code set via `runtime.setExitCode`.
    pub exit_code: Option<i32>,
}

impl Default for GraalVmState {
    fn default() -> Self {
        Self::new()
    }
}

impl AsMut<Self> for GraalVmState {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl GraalVmState {
    /// Creates a new initialized [`GraalVmState`].
    ///
    /// # Returns
    ///
    /// A [`GraalVmState`] pre-populated with global (`1`), `undefined` (`2`), and `null` (`3`)
    /// handles.
    #[must_use]
    pub fn new() -> Self {
        let mut heap = HashMap::new();
        heap.insert(1, "global".to_string());
        heap.insert(2, "undefined".to_string());
        heap.insert(3, "null".to_string());
        Self {
            heap,
            next_id: 4,
            stdout_buf: String::new(),
            stderr_buf: String::new(),
            exit_code: None,
        }
    }

    /// Inserts a string or object description into the virtual heap and returns its handle.
    ///
    /// # Arguments
    ///
    /// * `val` - The string representation of the JS object to store.
    ///
    /// # Returns
    ///
    /// The assigned `i32` handle on the simulated heap.
    pub fn insert_object(&mut self, val: String) -> i32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.heap.insert(id, val);
        id
    }

    /// Retrieves a reference to a virtual heap object by its handle.
    ///
    /// # Arguments
    ///
    /// * `id` - The `i32` handle previously returned by [`GraalVmState::insert_object`].
    ///
    /// # Returns
    ///
    /// `Some(&String)` if the handle exists, or `None` otherwise.
    #[must_use]
    pub fn get_object(&self, id: i32) -> Option<&String> {
        self.heap.get(&id)
    }
}

/// Helper to read a UTF-8 string from guest WebAssembly linear memory safely.
///
/// Validates that `ptr` and `len` are non-negative before reading from memory.
///
/// # Arguments
///
/// * `caller` - The active `wasmtime::Caller` context.
/// * `memory` - The guest `wasmtime::Memory` instance.
/// * `ptr` - Guest byte offset pointer (must be `>= 0`).
/// * `len` - Number of bytes to read (must be `>= 0`).
///
/// # Returns
///
/// The decoded UTF-8 `String` from linear memory.
///
/// # Errors
///
/// Returns [`CddEngineError::TryFromInt`] if `ptr` or `len` is negative,
/// [`CddEngineError::MemoryAccess`] if the range exceeds guest memory bounds, or
/// [`CddEngineError::Utf8`] if the bytes are not valid UTF-8.
pub fn read_string<T>(
    caller: &mut Caller<'_, T>,
    memory: &Memory,
    ptr: i32,
    len: i32,
) -> Result<String, CddEngineError> {
    let start = usize::try_from(ptr)?;
    let length = usize::try_from(len)?;
    let mut buffer = vec![0_u8; length];
    memory.read(caller, start, &mut buffer)?;
    let decoded = String::from_utf8(buffer)?;
    Ok(decoded)
}

/// Helper to write a UTF-8 string into guest WebAssembly linear memory safely.
///
/// Validates that `ptr` is non-negative before writing to memory.
///
/// # Arguments
///
/// * `caller` - The active `wasmtime::Caller` context.
/// * `memory` - The guest `wasmtime::Memory` instance.
/// * `s` - The string slice to write into linear memory.
/// * `ptr` - Guest byte offset pointer (must be `>= 0`).
///
/// # Returns
///
/// `Ok(())` when the bytes have been written into guest linear memory.
///
/// # Errors
///
/// Returns [`CddEngineError::TryFromInt`] if `ptr` is negative, or
/// [`CddEngineError::MemoryAccess`] if the write exceeds guest memory bounds.
pub fn write_string<T>(
    caller: &mut Caller<'_, T>,
    memory: &Memory,
    s: &str,
    ptr: i32,
) -> Result<(), CddEngineError> {
    let start = usize::try_from(ptr)?;
    memory.write(caller, start, s.as_bytes())?;
    Ok(())
}

/// Utility struct to attach `GraalVM` WebAssembly host imports to a [`wasmtime::Linker`].
pub struct GraalVmLinker;

impl GraalVmLinker {
    /// Registers `GraalVM` host functions (`env` and `wasm:js-string`) on the given linker.
    ///
    /// # Arguments
    ///
    /// * `linker` - The mutable [`wasmtime::Linker`] to populate.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all host imports were registered successfully.
    ///
    /// # Errors
    ///
    /// Returns [`CddEngineError::Wasmtime`] if registering any import fails.
    pub fn add_to_linker<T: AsMut<GraalVmState> + 'static>(
        linker: &mut Linker<T>,
    ) -> Result<(), CddEngineError> {
        Self::register_io_and_runtime(linker)?;
        Self::register_js_interop(linker)?;
        Self::register_jsbody_callbacks(linker)?;
        Self::register_wasm_js_string(linker)
    }

    /// Registers standard I/O writer, exit code, and math host imports under `"env"`.
    fn register_io_and_runtime<T: AsMut<GraalVmState> + 'static>(
        linker: &mut Linker<T>,
    ) -> Result<(), CddEngineError> {
        linker.func_wrap(
            "env",
            "stdoutWriter.printChars",
            |_caller: Caller<'_, T>, _chars: i32, _offset: i32, _len: i32| {},
        )?;
        linker.func_wrap(
            "env",
            "stderrWriter.printChars",
            |_caller: Caller<'_, T>, _chars: i32, _offset: i32, _len: i32| {},
        )?;
        linker.func_wrap("env", "stdoutWriter.flush", |_caller: Caller<'_, T>| {})?;
        linker.func_wrap("env", "stderrWriter.flush", |_caller: Caller<'_, T>| {})?;
        linker.func_wrap(
            "env",
            "runtime.setExitCode",
            |mut caller: Caller<'_, T>, code: i32| {
                caller.data_mut().as_mut().exit_code = Some(code);
            },
        )?;
        linker.func_wrap("env", "Math.cbrt", |x: f64| x.cbrt())?;
        linker.func_wrap("env", "Math.pow", |x: f64, y: f64| x.powf(y))?;
        Ok(())
    }

    /// Registers `interop.*` and `conversion.*` host imports under `"env"`.
    fn register_js_interop<T: 'static>(linker: &mut Linker<T>) -> Result<(), CddEngineError> {
        linker.func_wrap(
            "env",
            "conversion.stringToJava",
            |_caller: Caller<'_, T>, _js_str: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "conversion.javaToString",
            |_caller: Caller<'_, T>, _java_str: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "conversion.extractJavaObject",
            |_caller: Caller<'_, T>, _obj: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "conversion.boxBoolean",
            |_caller: Caller<'_, T>, val: i32| -> i32 { val },
        )?;
        linker.func_wrap(
            "env",
            "interop.getGlobal",
            |_caller: Caller<'_, T>| -> i32 { 1 },
        )?;
        linker.func_wrap(
            "env",
            "interop.getUndefined",
            |_caller: Caller<'_, T>| -> i32 { 2 },
        )?;
        linker.func_wrap("env", "interop.getNull", |_caller: Caller<'_, T>| -> i32 {
            3
        })?;
        Ok(())
    }

    /// Registers `jsbody.*` and `resource.read` host imports under `"env"`.
    fn register_jsbody_callbacks<T: 'static>(linker: &mut Linker<T>) -> Result<(), CddEngineError> {
        linker.func_wrap(
            "env",
            "jsbody.0",
            |_caller: Caller<'_, T>, _a: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "jsbody.1",
            |_caller: Caller<'_, T>, _a: i32, _b: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "jsbody.2",
            |_caller: Caller<'_, T>, _a: i32, _b: i32, _c: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "resource.read",
            |_caller: Caller<'_, T>, _path: i32, _buf: i32| -> i32 { -1 },
        )?;
        Ok(())
    }

    /// Registers `"wasm:js-string"` standard proposal host imports.
    fn register_wasm_js_string<T: 'static>(linker: &mut Linker<T>) -> Result<(), CddEngineError> {
        linker.func_wrap(
            "wasm:js-string",
            "fromCharCodeArray",
            |_caller: Caller<'_, T>, _arr: i32, _start: i32, _end: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "wasm:js-string",
            "intoCharCodeArray",
            |_caller: Caller<'_, T>, _str: i32, _arr: i32, _start: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "wasm:js-string",
            "length",
            |_caller: Caller<'_, T>, _str: i32| -> i32 { 0 },
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for [`GraalVmState`], [`GraalVmLinker`], [`read_string`], and [`write_string`].

    use super::*;
    use wasmtime::{Engine, Instance, MemoryType, Module, Store};

    /// Verifies [`GraalVmState`] initialization, object insertion, and retrieval.
    #[test]
    fn test_graalvm_state() {
        let mut state = GraalVmState::default();
        assert_eq!(state, GraalVmState::new());
        assert_eq!(state.get_object(1), Some(&"global".to_string()));
        assert_eq!(state.get_object(2), Some(&"undefined".to_string()));
        assert_eq!(state.get_object(3), Some(&"null".to_string()));
        assert_eq!(state.get_object(99), None);

        let id = state.insert_object("custom_obj".to_string());
        assert_eq!(id, 4);
        assert_eq!(state.get_object(4), Some(&"custom_obj".to_string()));
        assert!(format!("{state:?}").contains("GraalVmState"));
    }

    /// Verifies safe linear memory string reading, writing, and bounds/sign error handling.
    #[test]
    fn test_memory_read_write_and_validation() -> Result<(), CddEngineError> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, GraalVmState::new());
        let memory_ty = MemoryType::new(1, None);
        let memory = Memory::new(&mut store, memory_ty)?;

        let wat = r#"
            (module
                (import "env" "memory" (memory 1))
                (import "env" "test_rw" (func $test_rw))
                (import "env" "test_fail" (func $test_fail))
                (func (export "run")
                    call $test_rw
                )
                (func (export "run_fail")
                    call $test_fail
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        let module = Module::new(&engine, wasm_bytes)?;
        let mut linker = Linker::new(&engine);
        linker.define(&store, "env", "memory", memory)?;

        let mem_clone = memory;
        linker.func_wrap(
            "env",
            "test_rw",
            move |mut caller: Caller<'_, GraalVmState>| {
                let write_res = write_string(&mut caller, &mem_clone, "hello graalvm", 16);
                assert!(write_res.is_ok());
                let read_res = read_string(&mut caller, &mem_clone, 16, 13);
                assert_eq!(read_res.ok(), Some("hello graalvm".to_string()));
            },
        )?;

        let mem_clone2 = memory;
        linker.func_wrap(
            "env",
            "test_fail",
            move |mut caller: Caller<'_, GraalVmState>| {
                // Out of bounds write and read
                let oob_write = write_string(&mut caller, &mem_clone2, "hello", 100_000);
                assert!(oob_write.is_err());
                let oob_read = read_string(&mut caller, &mem_clone2, 100_000, 5);
                assert!(oob_read.is_err());

                // Negative pointer or length validation
                let neg_ptr_write = write_string(&mut caller, &mem_clone2, "hello", -1);
                assert!(neg_ptr_write.is_err());
                let neg_ptr_read = read_string(&mut caller, &mem_clone2, -1, 5);
                assert!(neg_ptr_read.is_err());
                let neg_len_read = read_string(&mut caller, &mem_clone2, 0, -5);
                assert!(neg_len_read.is_err());

                // Invalid UTF-8 bytes
                let _ = mem_clone2.write(&mut caller, 0, &[0xFF, 0xFF, 0xFF]);
                let bad_utf8 = read_string(&mut caller, &mem_clone2, 0, 3);
                assert!(bad_utf8.is_err());
            },
        )?;

        let instance = linker.instantiate(&mut store, &module)?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        run.call(&mut store, ())?;

        let run_fail = instance.get_typed_func::<(), ()>(&mut store, "run_fail")?;
        run_fail.call(&mut store, ())?;
        Ok(())
    }

    /// Verifies that duplicate linker registration returns a [`CddEngineError::Wasmtime`] error.
    #[test]
    fn test_graalvm_linker_duplicate_registration_errors() -> Result<(), CddEngineError> {
        let engine = Engine::default();

        let conflict_targets = [
            ("env", "stdoutWriter.printChars", 3),
            ("env", "stderrWriter.printChars", 3),
            ("env", "stdoutWriter.flush", 0),
            ("env", "stderrWriter.flush", 0),
            ("env", "runtime.setExitCode", 1),
            ("env", "Math.cbrt", 4),
            ("env", "Math.pow", 5),
            ("env", "conversion.stringToJava", 6),
            ("env", "conversion.javaToString", 6),
            ("env", "conversion.extractJavaObject", 6),
            ("env", "conversion.boxBoolean", 6),
            ("env", "interop.getGlobal", 7),
            ("env", "interop.getUndefined", 7),
            ("env", "interop.getNull", 7),
            ("env", "jsbody.0", 6),
            ("env", "jsbody.1", 8),
            ("env", "jsbody.2", 9),
            ("env", "resource.read", 8),
            ("wasm:js-string", "fromCharCodeArray", 9),
            ("wasm:js-string", "intoCharCodeArray", 9),
            ("wasm:js-string", "length", 6),
        ];

        for (module_name, func_name, sig_kind) in conflict_targets {
            let mut linker = Linker::<GraalVmState>::new(&engine);
            match sig_kind {
                0 => {
                    linker.func_wrap(module_name, func_name, || {})?;
                }
                1 => {
                    linker.func_wrap(module_name, func_name, |_: i32| {})?;
                }
                3 => {
                    linker.func_wrap(module_name, func_name, |_: i32, _: i32, _: i32| {})?;
                }
                4 => {
                    linker.func_wrap(module_name, func_name, |_: f64| -> f64 { 0.0 })?;
                }
                5 => {
                    linker.func_wrap(module_name, func_name, |_: f64, _: f64| -> f64 { 0.0 })?;
                }
                6 => {
                    linker.func_wrap(module_name, func_name, |_: i32| -> i32 { 0 })?;
                }
                7 => {
                    linker.func_wrap(module_name, func_name, || -> i32 { 0 })?;
                }
                8 => {
                    linker.func_wrap(module_name, func_name, |_: i32, _: i32| -> i32 { 0 })?;
                }
                _ => {
                    linker.func_wrap(module_name, func_name, |_: i32, _: i32, _: i32| -> i32 {
                        0
                    })?;
                }
            }
            assert!(GraalVmLinker::add_to_linker(&mut linker).is_err());
        }
        Ok(())
    }

    /// WebAssembly text fixture importing and calling every `GraalVM` host function.
    const GUEST_ALL_IMPORTS_WAT: &str = r#"
        (module
            (import "env" "stdoutWriter.printChars" (func $p_out (param i32 i32 i32)))
            (import "env" "stderrWriter.printChars" (func $p_err (param i32 i32 i32)))
            (import "env" "stdoutWriter.flush" (func $f_out))
            (import "env" "stderrWriter.flush" (func $f_err))
            (import "env" "runtime.setExitCode" (func $exit (param i32)))
            (import "env" "Math.cbrt" (func $cbrt (param f64) (result f64)))
            (import "env" "Math.pow" (func $pow (param f64 f64) (result f64)))
            (import "env" "conversion.stringToJava" (func $s2j (param i32) (result i32)))
            (import "env" "conversion.javaToString" (func $j2s (param i32) (result i32)))
            (import "env" "conversion.extractJavaObject" (func $ext (param i32) (result i32)))
            (import "env" "conversion.boxBoolean" (func $box (param i32) (result i32)))
            (import "env" "interop.getGlobal" (func $glob (result i32)))
            (import "env" "interop.getUndefined" (func $undef (result i32)))
            (import "env" "interop.getNull" (func $null (result i32)))
            (import "env" "jsbody.0" (func $jb0 (param i32) (result i32)))
            (import "env" "jsbody.1" (func $jb1 (param i32 i32) (result i32)))
            (import "env" "jsbody.2" (func $jb2 (param i32 i32 i32) (result i32)))
            (import "env" "resource.read" (func $rread (param i32 i32) (result i32)))
            (import "wasm:js-string" "fromCharCodeArray" (func $fcca (param i32 i32 i32) (result i32)))
            (import "wasm:js-string" "intoCharCodeArray" (func $icca (param i32 i32 i32) (result i32)))
            (import "wasm:js-string" "length" (func $slen (param i32) (result i32)))

            (func (export "test_all") (result i32)
                (call $p_out (i32.const 0) (i32.const 0) (i32.const 0))
                (call $p_err (i32.const 0) (i32.const 0) (i32.const 0))
                (call $f_out)
                (call $f_err)
                (call $exit (i32.const 0))
                (drop (call $cbrt (f64.const 8.0)))
                (drop (call $pow (f64.const 2.0) (f64.const 3.0)))
                (drop (call $s2j (i32.const 1)))
                (drop (call $j2s (i32.const 1)))
                (drop (call $ext (i32.const 1)))
                (drop (call $box (i32.const 1)))
                (drop (call $glob))
                (drop (call $undef))
                (drop (call $null))
                (drop (call $jb0 (i32.const 1)))
                (drop (call $jb1 (i32.const 1) (i32.const 2)))
                (drop (call $jb2 (i32.const 1) (i32.const 2) (i32.const 3)))
                (drop (call $rread (i32.const 1) (i32.const 2)))
                (drop (call $fcca (i32.const 1) (i32.const 2) (i32.const 3)))
                (drop (call $icca (i32.const 1) (i32.const 2) (i32.const 3)))
                (call $slen (i32.const 1))
            )
        )
    "#;

    /// Verifies invoking all `GraalVM` host imports from a compiled WebAssembly guest module.
    #[test]
    fn test_graalvm_linker_guest_execution() -> Result<(), CddEngineError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        GraalVmLinker::add_to_linker(&mut linker)?;

        let mut store = Store::new(&engine, GraalVmState::new());
        let wasm_bytes = wat::parse_str(GUEST_ALL_IMPORTS_WAT)
            .map_err(|e| CddEngineError::Wasm(e.to_string()))?;
        let module = Module::new(&engine, wasm_bytes)?;
        let imports: Vec<_> = module
            .imports()
            .filter_map(|imp| linker.get(&mut store, imp.module(), imp.name()).ok())
            .collect();
        let instance = Instance::new(&mut store, &module, &imports)?;
        let func = instance.get_typed_func::<(), i32>(&mut store, "test_all")?;
        let res = func.call(&mut store, ())?;
        assert_eq!(res, 0);
        assert_eq!(store.data().exit_code, Some(0));

        let math_pow = linker
            .get(&mut store, "env", "Math.pow")?
            .into_func()
            .ok_or_else(|| CddEngineError::Wasm("Math.pow missing".into()))?;
        let typed_pow = math_pow.typed::<(f64, f64), f64>(&store)?;
        let pow_val = typed_pow.call(&mut store, (2.0, 3.0))?;
        assert!((pow_val - 8.0).abs() < f64::EPSILON);

        Ok(())
    }
}
