use anyhow::{bail, Result};
use std::process::ExitCode;
use wasmtime::*;

fn main() -> Result<ExitCode> {
    let file = match std::env::args().nth(1) {
        Some(it) => it,
        None => {
            bail!("usage: webassembly-test-runner tests.wasm");
        }
    };
    // Modules can be compiled through either the text or binary format
    let engine = Engine::default();
    let module = Module::from_file(&engine, &file)?;
    let mut tests = Vec::new();
    for export in module.exports() {
        if let Some(name) = export.name().strip_prefix("$webassembly-test$") {
            let mut ignore = true;
            let name = name.strip_prefix("ignore$").unwrap_or_else(|| {
                ignore = false;
                name
            });
            tests.push((export, TestMeta { name, ignore }));
        }
    }
    let total = tests.len();

    eprintln!("\nrunning {} tests", total);
    let mut store = Store::new(&engine, ());
    let mut instance = Instance::new(&mut store, &module, &[])?;
    let mut passed = 0;
    let mut failed = 0;
    let mut ignored = 0;
    for (export, meta) in tests {
        eprint!("test {} ...", meta.name);
        if meta.ignore {
            ignored += 1;
            eprintln!(" ignored")
        } else {
            let f = instance.get_typed_func::<(), (), _>(&mut store, export.name())?;

            let result = f.call(&mut store, ());
            match result {
                Ok(_) => {
                    passed += 1;
                    eprintln!(" ok")
                }
                Err(e) => {
                    // Reset instance on test failure. WASM uses `panic=abort`, so
                    // `Drop`s are not called after test failures, and a failed test
                    // might leave an instance in an inconsistent state.
                    store = Store::new(&engine, ());
                    instance = Instance::new(&mut store, &module, &[])?;

                    failed += 1;
                    eprintln!(" FAILED");
                    eprintln!("{:?}", e)
                }
            }
        }
    }
    eprintln!(
        "\ntest result: {}. {} passed; {} failed; {} ignored;",
        if failed > 0 { "FAILED" } else { "ok" },
        passed,
        failed,
        ignored,
    );
    Ok(if failed > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}

struct TestMeta<'a> {
    name: &'a str,
    ignore: bool,
}
