//! GraalVM Linker bindings

use std::collections::HashMap;
use wasmtime::{Caller, Linker, Memory};

/// State object to hold JS mock references for GraalVM.
pub struct GraalVmState {
    /// Simulated JS heap for interop
    pub js_objects: HashMap<u32, Box<dyn std::any::Any + Send + Sync>>,
    next_id: u32,
}

impl Default for GraalVmState {
    fn default() -> Self {
        Self::new()
    }
}

impl GraalVmState {
    /// Creates a new `GraalVmState`.
    pub fn new() -> Self {
        Self {
            js_objects: HashMap::new(),
            next_id: 1,
        }
    }

    /// Inserts a mock JS object into state.
    pub fn insert_object(&mut self, obj: Box<dyn std::any::Any + Send + Sync>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.js_objects.insert(id, obj);
        id
    }

    /// Retrieves a mock JS object from state.
    pub fn get_object(&self, id: u32) -> Option<&(dyn std::any::Any + Send + Sync)> {
        self.js_objects.get(&id).map(|b| &**b)
    }
}

/// Helper to read a string from memory
pub fn read_string<T>(
    memory: &Memory,
    caller: &mut Caller<'_, T>,
    ptr: i32,
    len: i32,
) -> Result<String, String> {
    let mut buf = vec![0; len as usize];
    memory
        .read(caller, ptr as usize, &mut buf)
        .map_err(|e| e.to_string())?;
    String::from_utf8(buf).map_err(|e| e.to_string())
}

/// Helper to write a string to memory
pub fn write_string<T>(
    memory: &Memory,
    caller: &mut Caller<'_, T>,
    ptr: i32,
    s: &str,
) -> Result<(), String> {
    memory
        .write(caller, ptr as usize, s.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Linker implementation for GraalVM `jsbody` and `interop`.
pub struct GraalVmLinker;

impl GraalVmLinker {
    /// Links the stubs required by GraalVM into the given linker.
    pub fn add_to_linker<T: 'static + Send>(linker: &mut Linker<T>) -> Result<(), String> {
        linker
            .func_wrap(
                "interop",
                "stdoutWriter.printChars",
                |mut _caller: Caller<'_, T>, _ptr: i32, _len: i32| {
                    // Stub for Phase 3
                },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "interop",
                "stderrWriter.printChars",
                |mut _caller: Caller<'_, T>, _ptr: i32, _len: i32| {
                    // Stub for Phase 3
                },
            )
            .expect("Link error");

        linker
            .func_wrap("interop", "Date.now", |mut _caller: Caller<'_, T>| -> f64 {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
                    * 1000.0
            })
            .expect("Link error");

        linker
            .func_wrap(
                "interop",
                "performance.now",
                |mut _caller: Caller<'_, T>| -> f64 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64()
                        * 1000.0
                },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "interop",
                "runtime.setExitCode",
                |mut _caller: Caller<'_, T>, _code: i32| {
                    // Stub for Phase 3
                },
            )
            .expect("Link error");

        // Mock jsbody methods
        linker
            .func_wrap(
                "jsbody",
                "_JSObject.stringValue___String",
                |mut _caller: Caller<'_, T>| -> i32 { 0 },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "jsbody",
                "_JSNumber.javaDouble___Double",
                |mut _caller: Caller<'_, T>| -> f64 { 0.0 },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "jsbody",
                "_JSConversion.extractJavaScriptString___String_Object",
                |mut _caller: Caller<'_, T>| -> i32 { 0 },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "jsbody",
                "_JSObject.get___Object_Object",
                |mut _caller: Caller<'_, T>| -> i32 { 0 },
            )
            .expect("Link error");

        // Mock compat methods
        linker
            .func_wrap(
                "compat",
                "f64rem",
                |mut _caller: Caller<'_, T>, a: f64, b: f64| -> f64 { a % b },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "compat",
                "f64log",
                |mut _caller: Caller<'_, T>, a: f64| -> f64 { a.ln() },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "compat",
                "f64log10",
                |mut _caller: Caller<'_, T>, a: f64| -> f64 { a.log10() },
            )
            .expect("Link error");

        linker
            .func_wrap(
                "compat",
                "f64pow",
                |mut _caller: Caller<'_, T>, a: f64, b: f64| -> f64 { a.powf(b) },
            )
            .expect("Link error");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::{Config, Engine, Store};

    #[test]
    fn test_graalvm_state() {
        let mut state = GraalVmState::default();
        let id = state.insert_object(Box::new("test".to_string()));
        let obj = state
            .get_object(id)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            obj.downcast_ref::<String>()
                .expect("graalvm state expected to work in tests"),
            "test"
        );
        assert!(state.get_object(999).is_none());
    }

    #[test]
    fn test_memory_read_write() {
        let engine = Engine::new(&Config::new()).expect("graalvm state expected to work in tests");
        let mut store = Store::new(&engine, ());
        let memory = wasmtime::Memory::new(&mut store, wasmtime::MemoryType::new(1, None))
            .expect("graalvm state expected to work in tests");

        let mut linker = Linker::<()>::new(&engine);
        linker
            .func_wrap(
                "env",
                "test",
                move |mut caller: wasmtime::Caller<'_, ()>| {
                    write_string(&memory, &mut caller, 10, "hello")
                        .expect("graalvm state expected to work in tests");
                    let s = read_string(&memory, &mut caller, 10, 5)
                        .expect("graalvm state expected to work in tests");
                    assert_eq!(s, "hello");

                    assert!(read_string(&memory, &mut caller, 65536, 1).is_err());
                    assert!(write_string(&memory, &mut caller, 65536, "a").is_err());

                    memory
                        .write(&mut caller, 0, &[0xff, 0xff])
                        .expect("graalvm state expected to work in tests");
                    assert!(read_string(&memory, &mut caller, 0, 2).is_err());
                },
            )
            .expect("graalvm state expected to work in tests");

        let func = linker
            .get(&mut store, "env", "test")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), ()>(&store)
            .expect("graalvm state expected to work in tests");
        func.call(&mut store, ())
            .expect("graalvm state expected to work in tests");
    }
    #[test]
    fn test_graalvm_linker() {
        let engine = Engine::new(&Config::new()).expect("graalvm state expected to work in tests");
        let mut linker = Linker::<()>::new(&engine);
        GraalVmLinker::add_to_linker(&mut linker).expect("graalvm state expected to work in tests");

        let mut store = Store::new(&engine, ());

        let func = linker
            .get(&mut store, "interop", "stdoutWriter.printChars")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(i32, i32), ()>(&store)
            .expect("graalvm state expected to work in tests");
        func.call(&mut store, (0, 0))
            .expect("graalvm state expected to work in tests");

        let func = linker
            .get(&mut store, "interop", "stderrWriter.printChars")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(i32, i32), ()>(&store)
            .expect("graalvm state expected to work in tests");
        func.call(&mut store, (0, 0))
            .expect("graalvm state expected to work in tests");

        let func = linker
            .get(&mut store, "interop", "Date.now")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), f64>(&store)
            .expect("graalvm state expected to work in tests");
        let res = func
            .call(&mut store, ())
            .expect("graalvm state expected to work in tests");
        assert!(res > 0.0);

        let func = linker
            .get(&mut store, "interop", "performance.now")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), f64>(&store)
            .expect("graalvm state expected to work in tests");
        let res = func
            .call(&mut store, ())
            .expect("graalvm state expected to work in tests");
        assert!(res > 0.0);

        let func = linker
            .get(&mut store, "interop", "runtime.setExitCode")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(i32,), ()>(&store)
            .expect("graalvm state expected to work in tests");
        func.call(&mut store, (0,))
            .expect("graalvm state expected to work in tests");

        let func = linker
            .get(&mut store, "jsbody", "_JSObject.stringValue___String")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), i32>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, ())
                .expect("graalvm state expected to work in tests"),
            0
        );

        let func = linker
            .get(&mut store, "jsbody", "_JSNumber.javaDouble___Double")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), f64>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, ())
                .expect("graalvm state expected to work in tests"),
            0.0
        );

        let func = linker
            .get(
                &mut store,
                "jsbody",
                "_JSConversion.extractJavaScriptString___String_Object",
            )
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), i32>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, ())
                .expect("graalvm state expected to work in tests"),
            0
        );

        let func = linker
            .get(&mut store, "jsbody", "_JSObject.get___Object_Object")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(), i32>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, ())
                .expect("graalvm state expected to work in tests"),
            0
        );

        let func = linker
            .get(&mut store, "compat", "f64rem")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(f64, f64), f64>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, (5.0, 2.0))
                .expect("graalvm state expected to work in tests"),
            1.0
        );

        let func = linker
            .get(&mut store, "compat", "f64log")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(f64,), f64>(&store)
            .expect("graalvm state expected to work in tests");
        assert!(
            func.call(&mut store, (10.0,))
                .expect("graalvm state expected to work in tests")
                > 2.0
        );

        let func = linker
            .get(&mut store, "compat", "f64log10")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(f64,), f64>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, (10.0,))
                .expect("graalvm state expected to work in tests"),
            1.0
        );

        let func = linker
            .get(&mut store, "compat", "f64pow")
            .expect("graalvm state expected to work in tests")
            .into_func()
            .expect("graalvm state expected to work in tests")
            .typed::<(f64, f64), f64>(&store)
            .expect("graalvm state expected to work in tests");
        assert_eq!(
            func.call(&mut store, (2.0, 3.0))
                .expect("graalvm state expected to work in tests"),
            8.0
        );
    }
}
