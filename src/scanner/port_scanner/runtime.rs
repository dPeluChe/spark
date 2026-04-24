//! Runtime detection from process name + command line, and project directory
//! resolution from cwd / cmdline.

use super::Runtime;
use std::path::PathBuf;

pub fn detect_runtime(process_name: &str, cmdline: &str) -> Runtime {
    let name = process_name.to_lowercase();
    let cmd = cmdline.to_lowercase();

    if name == "node"
        || name.starts_with("node ")
        || cmd.contains("node ")
        || cmd.contains("ts-node")
        || cmd.contains("tsx ")
        || cmd.contains("next ")
        || cmd.contains("vite")
        || cmd.contains("webpack")
        || cmd.contains("esbuild")
        || cmd.contains("npm ")
        || cmd.contains("npx ")
        || cmd.contains("yarn ")
        || cmd.contains("pnpm ")
    {
        return Runtime::Node;
    }

    if name == "bun" || cmd.contains("bun ") {
        return Runtime::Bun;
    }

    if name == "deno" || cmd.contains("deno ") {
        return Runtime::Deno;
    }

    if name.starts_with("python")
        || name == "uvicorn"
        || name == "gunicorn"
        || name == "flask"
        || name == "django"
        || name == "celery"
        || name == "jupyter"
        || name == "ipython"
        || cmd.contains("python")
        || cmd.contains("uvicorn")
        || cmd.contains("gunicorn")
        || cmd.contains("flask")
        || cmd.contains("manage.py")
        || cmd.contains("jupyter")
    {
        return Runtime::Python;
    }

    if name == "ruby"
        || name == "puma"
        || name == "unicorn"
        || name == "rails"
        || cmd.contains("ruby")
        || cmd.contains("rails ")
        || cmd.contains("puma")
        || cmd.contains("unicorn")
        || cmd.contains("bundle exec")
    {
        return Runtime::Ruby;
    }

    if name == "java"
        || name.starts_with("java ")
        || cmd.contains("java ")
        || cmd.contains("spring")
        || cmd.contains("gradle")
        || cmd.contains("mvn")
        || cmd.contains(".jar")
    {
        return Runtime::Java;
    }

    if name == "dotnet" || cmd.contains("dotnet ") || cmd.contains(".dll") {
        return Runtime::Dotnet;
    }

    if name == "php"
        || name == "php-fpm"
        || cmd.contains("php ")
        || cmd.contains("artisan")
        || cmd.contains("composer")
    {
        return Runtime::Php;
    }

    if name == "beam.smp"
        || name == "elixir"
        || cmd.contains("mix ")
        || cmd.contains("phoenix")
        || cmd.contains("elixir")
    {
        return Runtime::Elixir;
    }

    // Rust before Go to avoid "cargo run" matching "go run"
    if cmd.contains("cargo run") || cmd.contains("cargo watch") || cmd.contains("cargo ") {
        return Runtime::Rust;
    }

    if is_go_binary(process_name) || cmd.contains("go run") {
        return Runtime::Go;
    }

    if name == "nginx" || name.starts_with("nginx") {
        return Runtime::Nginx;
    }

    if name == "docker-proxy" || name == "containerd" || name.starts_with("docker") {
        return Runtime::Docker;
    }

    // macOS system services
    if name == "controlce" || name == "controlcenter" || name == "rapportd" {
        return Runtime::Other("macOS".into());
    }
    if name == "spotify" {
        return Runtime::Other("Spotify".into());
    }
    if name.contains("redis") {
        return Runtime::Other("Redis".into());
    }
    if name.contains("postgres") || name.contains("postmaster") {
        return Runtime::Other("PostgreSQL".into());
    }
    if name.contains("mysql") || name.contains("mariadbd") {
        return Runtime::Other("MySQL".into());
    }
    if name.contains("mongo") {
        return Runtime::Other("MongoDB".into());
    }
    if name.contains("figma") {
        return Runtime::Other("Figma".into());
    }

    Runtime::Other(process_name.to_string())
}

fn is_go_binary(process_name: &str) -> bool {
    let go_hints = ["air", "gin", "fiber", "echo", "mux"];
    let name_lower = process_name.to_lowercase();
    go_hints.iter().any(|h| name_lower == *h)
}

/// Best-effort project directory: nearest .git parent of the cwd,
/// or fallback to a path-like cmdline arg.
pub fn resolve_project_dir(cwd: &Option<PathBuf>, cmdline: &str) -> Option<String> {
    if let Some(dir) = cwd {
        let mut check = dir.clone();
        loop {
            if check.join(".git").exists() {
                let name = check
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let path = check.display().to_string();
                let home = std::env::var("HOME").unwrap_or_default();
                let display = if path.starts_with(&home) {
                    format!("~{}", &path[home.len()..])
                } else {
                    path
                };
                return Some(format!("{} ({})", name, display));
            }
            if !check.pop() {
                break;
            }
        }

        let path = dir.display().to_string();
        let home = std::env::var("HOME").unwrap_or_default();
        if path.starts_with(&home) {
            return Some(format!("~{}", &path[home.len()..]));
        }
        return Some(path);
    }

    for part in cmdline.split_whitespace() {
        if part.starts_with('/') || part.starts_with("./") {
            return Some(part.to_string());
        }
    }

    None
}
