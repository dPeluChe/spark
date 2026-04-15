//! System/setup CLI commands: init, config, doctor, agent, completions.

use std::io;
use std::path::PathBuf;
use clap::CommandFactory;
use crate::config;
use crate::scanner;
use super::Cli;

pub fn cmd_root(set: Option<PathBuf>, config: &mut config::SparkConfig) -> color_eyre::Result<()> {
    if let Some(new_root) = set {
        let path = if new_root.starts_with("~") {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(new_root.strip_prefix("~/").unwrap_or(&new_root))
        } else {
            std::fs::canonicalize(&new_root).unwrap_or(new_root)
        };
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            println!("  Created: {}", path.display());
        }
        config.repos_root = path.clone();
        config.save()?;
        println!("  repos_root set to: {}", path.display());
        println!("  Saved to ~/.config/spark/config.toml");
    } else {
        println!("{}", config.repos_root.display());
    }
    Ok(())
}

pub fn cmd_init(config: &config::SparkConfig) -> color_eyre::Result<()> {
    println!("  Spark Init — Setting up your environment\n");

    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") {
        dirs::home_dir().unwrap_or_default().join(".zshrc")
    } else if shell.contains("bash") {
        dirs::home_dir().unwrap_or_default().join(".bashrc")
    } else {
        dirs::home_dir().unwrap_or_default().join(".profile")
    };
    println!("  Shell: {}", shell);
    println!("  Config: {}\n", rc_file.display());

    let rc_content = std::fs::read_to_string(&rc_file).unwrap_or_default();
    let has_spark_cd = rc_content.contains("spark-cd");

    let spark_block = r#"
# --- Spark DevOps Platform ---
# Navigate to managed repos
spark-cd() { cd "$(spark cd "$1")" ; }
# --- End Spark ---"#;

    if has_spark_cd {
        println!("  [+] spark-cd already configured");
    } else {
        print!("  Add spark-cd to {}? (y/N): ", rc_file.display());
        io::Write::flush(&mut io::stdout())?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim().to_lowercase() == "y" {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().append(true).open(&rc_file)?;
            writeln!(f, "{}", spark_block)?;
            println!("  [+] Added spark-cd to {}", rc_file.display());
        } else {
            println!("  [-] Skipped");
        }
    }

    let config_dir = dirs::config_dir().unwrap_or_default().join("spark");
    std::fs::create_dir_all(&config_dir)?;

    let whitelist_path = config_dir.join("whitelist.txt");
    if !whitelist_path.exists() {
        std::fs::write(&whitelist_path,
            "# Spark whitelist — paths listed here will be skipped during system cleanup\n\
             # One path per line. Use ~ for home directory.\n"
        )?;
        println!("  [+] Created {}", whitelist_path.display());
    } else {
        println!("  [+] Whitelist already exists");
    }

    let home = dirs::home_dir().unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("");
    match shell_name {
        "zsh" => {
            let comp_dir = home.join(".zsh/completions");
            let _ = std::fs::create_dir_all(&comp_dir);
            let comp_file = comp_dir.join("_spark");
            if !comp_file.exists() {
                let mut buf = Vec::new();
                clap_complete::generate(clap_complete::Shell::Zsh, &mut Cli::command(), "spark", &mut buf);
                let _ = std::fs::write(&comp_file, buf);
                println!("  [+] Installed zsh completions");
            } else {
                println!("  [+] Zsh completions already installed");
            }
        }
        "bash" => {
            let comp_dir = home.join(".local/share/bash-completion/completions");
            let _ = std::fs::create_dir_all(&comp_dir);
            let comp_file = comp_dir.join("spark");
            if !comp_file.exists() {
                let mut buf = Vec::new();
                clap_complete::generate(clap_complete::Shell::Bash, &mut Cli::command(), "spark", &mut buf);
                let _ = std::fs::write(&comp_file, buf);
                println!("  [+] Installed bash completions");
            } else {
                println!("  [+] Bash completions already installed");
            }
        }
        _ => {}
    }

    // Install AI agent skill — writes to ~/.agents/skills/spark/SKILL.md
    // then symlinks from ~/.claude/skills/spark, ~/.codex/skills/spark, ~/.gemini/skills/spark
    let skill_content = include_str!("../../assets/spark.skill.md");
    let agents_skill_dir = home.join(".agents/skills/spark");
    let skill_installed = if !agents_skill_dir.exists() {
        std::fs::create_dir_all(&agents_skill_dir).ok();
        let skill_file = agents_skill_dir.join("SKILL.md");
        std::fs::write(&skill_file, skill_content).is_ok()
    } else {
        // Update skill content on re-init
        let skill_file = agents_skill_dir.join("SKILL.md");
        std::fs::write(&skill_file, skill_content).is_ok()
    };

    if skill_installed {
        println!("  [+] Skill: {}", agents_skill_dir.join("SKILL.md").display());
        // Symlink from each AI agent's skills dir
        let link_dirs = [
            home.join(".claude/skills"),
            home.join(".codex/skills"),
            home.join(".gemini/skills"),
        ];
        for link_dir in &link_dirs {
            if link_dir.exists() {
                let link = link_dir.join("spark");
                if !link.exists() {
                    // Relative symlink: ../../.agents/skills/spark
                    let _ = std::os::unix::fs::symlink(&agents_skill_dir, &link);
                    println!("  [+] Linked skill: {}", link.display());
                } else {
                    println!("  [+] Skill link exists: {}", link.display());
                }
            }
        }
    } else {
        println!("  [-] Could not install skill (check ~/.agents/skills/ permissions)");
    }

    println!("\n  Setup complete!\n");
    println!("  Repos root:  {}", config.repos_root.display());
    println!("  Config:      {}", config_dir.display());
    println!("  Whitelist:   {}", whitelist_path.display());
    println!("\n  Run `source {}` to activate, then:", rc_file.display());
    println!("    spark-cd <repo>    Navigate to a repo");
    println!("    spark              Open the TUI");
    println!("    spark clone <url>  Clone a repo");
    Ok(())
}

pub fn cmd_agent(config: &config::SparkConfig) {
    let root = config.repos_root.display();
    println!("  Spark Repo Manager — Agent Integration\n");
    println!("  Your repos live at: {}\n", root);
    println!("  For AI agents (Claude Code, Cursor, Codex):\n");
    println!("  1. Tell your agent:");
    println!("     \"Repos managed by spark. Run `spark cd <name>` to find paths.\"\n");
    println!("  2. Add to CLAUDE.md or .cursorrules:");
    println!("     Repos root: {}", root);
    println!("     Find repo: spark cd <name>\n");
    println!("  3. Shell function (add to ~/.zshrc):");
    println!("     spark-cd() {{ cd \"$(spark cd \"$1\")\" ; }}\n");
    println!("  4. Commands:");
    println!("     spark cd zed           # Print path to zed repo");
    println!("     spark search zed       # Search all matching repos");
    println!("     spark list -p          # All repo paths");
    println!("     spark clone user/repo  # Clone to managed root");
    println!("     spark root             # Show repos root path");
}

pub fn cmd_completions(shell: clap_complete::Shell) {
    clap_complete::generate(shell, &mut Cli::command(), "spark", &mut io::stdout());
}

pub fn cmd_config(key: Option<String>, set: Option<String>, config: &mut config::SparkConfig) -> color_eyre::Result<()> {
    if let Some(value) = set {
        let key = key.unwrap_or_default();
        match key.as_str() {
            "repos_root" => { config.repos_root = PathBuf::from(&value); config.save()?; println!("  repos_root = {}", value); }
            "stale_threshold_days" => {
                config.stale_threshold_days = value.parse().map_err(|_| color_eyre::eyre::eyre!("Invalid number: {}", value))?;
                config.save()?; println!("  stale_threshold_days = {}", value);
            }
            "max_scan_depth" => {
                config.max_scan_depth = value.parse().map_err(|_| color_eyre::eyre::eyre!("Invalid number: {}", value))?;
                config.save()?; println!("  max_scan_depth = {}", value);
            }
            "use_trash" => {
                config.use_trash = value.parse().map_err(|_| color_eyre::eyre::eyre!("Expected true/false: {}", value))?;
                config.save()?; println!("  use_trash = {}", value);
            }
            _ => {
                eprintln!("  Unknown key: {}", key);
                eprintln!("  Available: repos_root, stale_threshold_days, max_scan_depth, use_trash");
                std::process::exit(1);
            }
        }
        println!("  Saved to ~/.config/spark/config.toml");
    } else {
        println!("  repos_root          = {}", config.repos_root.display());
        println!("  stale_threshold_days = {}", config.stale_threshold_days);
        println!("  large_artifact_threshold = {} bytes", config.large_artifact_threshold);
        println!("  use_trash           = {}", config.use_trash);
        println!("  max_scan_depth      = {}", config.max_scan_depth);
        println!("  scan_directories:");
        for dir in &config.scan_directories { println!("    - {}", dir.display()); }
        println!("\n  Config file: ~/.config/spark/config.toml");
        if let Some(key) = key { println!("\n  Tip: spark config {} --set <value>", key); }
    }
    Ok(())
}

pub fn cmd_doctor(config: &config::SparkConfig) {
    println!("\n  SPARK Doctor — Environment Health Check\n");

    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut warn = 0u32;

    let mut check = |label: &str, ok: bool, fix: &str| {
        if ok {
            println!("  \x1b[32m+\x1b[0m {}", label); pass += 1;
        } else {
            println!("  \x1b[31mx\x1b[0m {}  \x1b[90m-> {}\x1b[0m", label, fix); fail += 1;
        }
    };

    let version = env!("CARGO_PKG_VERSION");
    check("spark binary", true, "");
    println!("    version: {}", version);
    println!("    path:    {}", std::env::current_exe().unwrap_or_default().display());

    let git_ok = std::process::Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
    check("git available", git_ok, "install git: brew install git");

    let root_exists = config.repos_root.exists();
    let root_writable = root_exists && std::fs::metadata(&config.repos_root).map(|m| !m.permissions().readonly()).unwrap_or(false);
    check(&format!("repos root exists ({})", config.repos_root.display()), root_exists, &format!("mkdir -p {}", config.repos_root.display()));
    if root_exists {
        check("repos root writable", root_writable, "check permissions");
        println!("    repos:   {}", scanner::repo_manager::list_managed_repos(&config.repos_root).len());
    }

    let config_dir = dirs::config_dir().unwrap_or_default().join("spark");
    check("config directory", config_dir.exists(), &format!("spark init  (creates {})", config_dir.display()));
    check("config.toml", config_dir.join("config.toml").exists(), "spark init  (creates default config)");
    check("whitelist.txt", config_dir.join("whitelist.txt").exists(), "spark init  (creates whitelist)");

    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") { dirs::home_dir().unwrap_or_default().join(".zshrc") }
        else if shell.contains("bash") { dirs::home_dir().unwrap_or_default().join(".bashrc") }
        else { dirs::home_dir().unwrap_or_default().join(".profile") };
    let has_spark_cd = std::fs::read_to_string(&rc_file).unwrap_or_default().contains("spark-cd");
    check(&format!("spark-cd in {}", rc_file.file_name().unwrap_or_default().to_string_lossy()), has_spark_cd, "spark init  (adds spark-cd function)");

    let home = dirs::home_dir().unwrap_or_default();
    let has_completions = if shell.contains("zsh") { home.join(".zsh/completions/_spark").exists() }
        else if shell.contains("bash") { home.join(".local/share/bash-completion/completions/spark").exists() }
        else { false };
    check("shell completions", has_completions, "spark init  (installs completions)");

    if cfg!(target_os = "macos") {
        let lsof_ok = std::process::Command::new("lsof").arg("-v").output().map(|_| true).unwrap_or(false);
        check("lsof (port scanner)", lsof_ok, "should be pre-installed on macOS");

        let docker_ok = std::process::Command::new("docker").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
        if docker_ok { println!("  \x1b[32m+\x1b[0m docker available (system cleanup)"); pass += 1; }
        else { println!("  \x1b[33m~\x1b[0m docker not found  \x1b[90m-> optional, needed for Docker cleanup\x1b[0m"); warn += 1; }
    }

    let cache_path = dirs::config_dir().unwrap_or_default().join("spark").join("repo_status_cache.json");
    if cache_path.exists() {
        let age = std::fs::metadata(&cache_path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).map(|d| d.as_secs() / 3600);
        match age {
            Some(h) if h < 4 => { println!("  \x1b[32m+\x1b[0m status cache ({}h old, valid)", h); pass += 1; }
            Some(h) => { println!("  \x1b[33m~\x1b[0m status cache ({}h old, expired)  \x1b[90m-> spark status to refresh\x1b[0m", h); warn += 1; }
            None => { println!("  \x1b[33m~\x1b[0m status cache (age unknown)"); warn += 1; }
        }
    } else {
        println!("  \x1b[33m~\x1b[0m no status cache  \x1b[90m-> spark status to create\x1b[0m"); warn += 1;
    }

    println!("\n  ---------------------------------");
    println!("  {} passed   {} failed   {} warnings", pass, fail, warn);
    if fail == 0 { println!("\n  \x1b[32mSpark is healthy!\x1b[0m\n"); }
    else { println!("\n  Run \x1b[1mspark init\x1b[0m to fix most issues.\n"); }
}
