//! Read-only native inventory; output is always incomplete reference evidence.
#[path = "implementation_inventory/core.rs"]
mod core;
use std::{env, fs, path::Path};
fn collect(root: &Path, dir: &Path, files: &mut Vec<core::FileInput>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("symlink in implementation root is unsupported".into());
        }
        let path = entry.path();
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("yaml" | "yml")
        ) {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non-UTF8 path")?
                .replace('\\', "/");
            files.push(core::FileInput {
                path: relative,
                bytes: fs::read(path).map_err(|e| e.to_string())?,
            });
        }
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let mut a: Vec<_> = env::args_os().skip(1).collect();
    let references = a.len() == 4 && a.last().is_some_and(|v| v == "--references");
    if references {
        a.pop();
    }
    if a.len() != 3 {
        return Err("usage: implementation_inventory <E-array.json> <YAML-root> <new-output.json> [--references]".into());
    }
    let ids: Vec<String> = serde_json::from_slice(&fs::read(&a[0]).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let root = Path::new(&a[1]);
    if fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("symlink root unsupported".into());
    }
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    let payload = if references {
        serde_json::to_vec(&core::build_inventory_with_references(ids, files)?)
    } else {
        serde_json::to_vec(&core::build_inventory(ids, files)?)
    }
    .map_err(|e| e.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[2])
        .map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut output, &payload).map_err(|e| e.to_string())?;
    Ok(())
}
// The OS main-thread stack on Windows is too small for the recursive YAML
// parser/compiler. RUST_MIN_STACK does not resize main. Keep the entire existing
// operation (including serialization and drops) on one explicitly bounded stack.
const INVENTORY_STACK_BYTES: usize = 256 * 1024 * 1024;

fn run_with_stack(operation: fn() -> Result<(), String>) -> Result<(), String> {
    let worker = std::thread::Builder::new()
        .name("implementation-inventory".into())
        .stack_size(INVENTORY_STACK_BYTES)
        .spawn(operation)
        .map_err(|error| format!("failed to spawn implementation inventory worker: {error}"))?;
    match worker.join() {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn main() {
    if let Err(e) = run_with_stack(run) {
        eprintln!("{e}");
        std::process::exit(1)
    }
}

#[cfg(test)]
mod inventory_worker_tests {
    use super::run_with_stack;

    #[test]
    fn real_work_runs_on_named_worker() {
        run_with_stack(|| {
            assert_eq!(std::thread::current().name(), Some("implementation-inventory"));
            Ok(())
        }).unwrap();
    }

    #[test]
    fn ordinary_error_is_not_success_or_rewritten() {
        assert_eq!(run_with_stack(|| Err("original inventory error".into())),
                   Err("original inventory error".into()));
    }

    #[test]
    fn worker_panic_keeps_payload_and_is_not_success() {
        let panic = std::panic::catch_unwind(|| run_with_stack(|| {
            std::panic::panic_any(6813_u32);
        })).unwrap_err();
        assert_eq!(panic.downcast_ref::<u32>(), Some(&6813_u32));
    }

    #[test]
    fn worker_has_capacity_beyond_windows_main_stack() {
        #[inline(never)]
        fn work() -> Result<(), String> {
            let mut bytes = [0_u8; 2 * 1024 * 1024];
            bytes[0] = 3;
            bytes[bytes.len() - 1] = 7;
            let observed = std::hint::black_box(&mut bytes);
            assert_eq!(observed[0] + observed[observed.len() - 1], 10);
            Ok(())
        }
        run_with_stack(work).unwrap();
    }
}
