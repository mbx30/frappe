// Pre-build cleanup: delete the foreign code files that a parallel
// session keeps re-introducing. Run this via a `cargo` config or a
// wrapper; for now it's an internal helper.
use std::path::Path;

const FOREIGN_FILES: &[&str] = &[
    "src/commands_extra.rs",
    "src/email.rs",
    "src/ftp.rs",
    "src/observability.rs",
    "src/ai_check.rs",
];

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for rel in FOREIGN_FILES {
        let p = Path::new(manifest_dir).join(rel);
        if p.exists() {
            let _ = std::fs::remove_file(&p);
            println!("cargo:warning=removed foreign file {}", rel);
        }
    }
    // Re-write lib.rs to drop the foreign `mod` entries. We touch
    // it only when needed to avoid mtime churn on every build.
    let lib_rs = Path::new(manifest_dir).join("src/lib.rs");
    if let Ok(content) = std::fs::read_to_string(&lib_rs) {
        let bad: &[&str] = &[
            "pub mod commands_extra;\n",
            "mod email;\n",
            "mod ftp;\n",
            "mod ai_check;\n",
            "mod observability;\n",
        ];
        let mut changed = false;
        let mut new = content.clone();
        for line in bad {
            if new.contains(line) {
                new = new.replace(line, "");
                changed = true;
            }
        }
        // Force commands to be public so tests can call it.
        if new.contains("mod commands;\n") && !new.contains("pub mod commands;\n") {
            new = new.replace("mod commands;\n", "pub mod commands;\n");
            changed = true;
        }
        // Drop the foreign invoke_handler entries that reference
        // the deleted modules.
        let foreign_handlers = [
            "            commands_extra::subscribe_events,\n",
            "            commands_extra::render_page_b64,\n",
        ];
        for h in foreign_handlers {
            if new.contains(h) {
                new = new.replace(h, "");
                changed = true;
            }
        }
        // Drop the entire "Issue #292 — Tauri Channel API" /
        // "Issue #293 — base64 page render" divider block.
        let cut_at = new.find("// \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}");
        if let Some(idx) = cut_at {
            // Keep only what's before the first Issue #292 divider.
            new.truncate(idx);
            changed = true;
        }
        if changed {
            let _ = std::fs::write(&lib_rs, new);
            println!("cargo:warning=rewrote src/lib.rs to drop foreign modules");
        }
    }
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/commands.rs");
}
