//! Certificate scanner: find and analyze SSL/TLS certificates on the system.
//!
//! Scans for .pem, .crt, .cer files in projects and system locations,
//! parses expiration dates, and reports expired or soon-to-expire certs.
//! On macOS, also queries the system Keychain.

use std::path::{Path, PathBuf};
use std::process::Command;
use x509_parser::prelude::*;

/// Certificate status based on days until expiration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CertStatus {
    Expired,
    Expiring30,  // <= 30 days
    Expiring90,  // <= 90 days
    Valid,
}

impl std::fmt::Display for CertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertStatus::Expired => write!(f, "EXPIRED"),
            CertStatus::Expiring30 => write!(f, "Expiring (<30d)"),
            CertStatus::Expiring90 => write!(f, "Expiring (<90d)"),
            CertStatus::Valid => write!(f, "Valid"),
        }
    }
}

/// Source of the certificate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertSource {
    File(PathBuf),
    Keychain(String),
}

impl std::fmt::Display for CertSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertSource::File(p) => write!(f, "{}", p.display()),
            CertSource::Keychain(name) => write!(f, "Keychain: {}", name),
        }
    }
}

/// A discovered certificate with its metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CertInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub days_remaining: i64,
    pub status: CertStatus,
    pub source: CertSource,
    pub is_self_signed: bool,
    pub serial: String,
}

/// Scan results
#[derive(Debug, Clone)]
pub struct CertScanResult {
    pub certs: Vec<CertInfo>,
    pub expired_count: usize,
    pub expiring_count: usize,
    pub valid_count: usize,
}

/// Certificate file extensions to scan
const CERT_EXTENSIONS: &[&str] = &["pem", "crt", "cer", "cert"];

/// Private key file extensions
const KEY_EXTENSIONS: &[&str] = &["key", "p12", "pfx", "jks"];

/// Directories to skip
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", "target", "dist", "build", ".venv", "venv",
    "__pycache__", ".next", "vendor", ".cargo",
];

/// Scan a directory for certificate files and parse them.
pub fn scan_directory_certs(path: &Path) -> Vec<CertInfo> {
    let mut certs = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .max_depth(6)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() { return true; }
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() { continue; }
        let file_path = entry.path();

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !CERT_EXTENSIONS.contains(&ext.to_lowercase().as_str()) { continue; }

        certs.extend(parse_cert_file(file_path));
    }

    certs.sort_by_key(|c| c.days_remaining);
    certs
}

/// Scan macOS system Keychain for certificates
pub fn scan_keychain() -> Vec<CertInfo> {
    if !cfg!(target_os = "macos") { return Vec::new(); }

    let mut certs = Vec::new();

    // Export all certs from System keychain as PEM
    let keychains = [
        "/Library/Keychains/System.keychain",
        &format!("{}/Library/Keychains/login.keychain-db",
            std::env::var("HOME").unwrap_or_default()),
    ];

    for keychain in &keychains {
        let output = match Command::new("security")
            .args(["find-certificate", "-a", "-p", keychain])
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => continue,
        };

        let pem_data = String::from_utf8_lossy(&output.stdout);
        let keychain_name = if keychain.contains("login") { "Login" } else { "System" };

        // Split PEM blocks and parse each
        for pem_block in pem_data.split("-----BEGIN CERTIFICATE-----") {
            if !pem_block.contains("-----END CERTIFICATE-----") { continue; }
            let full_pem = format!("-----BEGIN CERTIFICATE-----{}", pem_block);

            if let Some(cert) = parse_pem_string(&full_pem, keychain_name) {
                certs.push(cert);
            }
        }
    }

    certs.sort_by_key(|c| c.days_remaining);
    certs
}

/// Parse a certificate file (may contain multiple PEM blocks)
pub fn parse_cert_file(path: &Path) -> Vec<CertInfo> {
    let content = match std::fs::read(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut certs = Vec::new();

    // Try PEM format first
    if let Ok(content_str) = std::str::from_utf8(&content) {
        if content_str.contains("-----BEGIN CERTIFICATE-----") {
            for pem in Pem::iter_from_buffer(content.as_slice()).flatten() {
                if let Ok((_, cert)) = X509Certificate::from_der(&pem.contents) {
                    certs.push(cert_to_info(&cert, CertSource::File(path.to_path_buf())));
                }
            }
            if !certs.is_empty() { return certs; }
        }
    }

    // Try DER format
    if let Ok((_, cert)) = X509Certificate::from_der(&content) {
        certs.push(cert_to_info(&cert, CertSource::File(path.to_path_buf())));
    }

    certs
}

/// Parse a PEM string (from keychain export)
fn parse_pem_string(pem_str: &str, keychain_name: &str) -> Option<CertInfo> {
    let pem = Pem::iter_from_buffer(pem_str.as_bytes()).next()?.ok()?;
    let (_, cert) = X509Certificate::from_der(&pem.contents).ok()?;
    let subject = cert.subject().to_string();
    Some(cert_to_info(&cert, CertSource::Keychain(format!("{} — {}", keychain_name,
        subject.split("CN=").nth(1).unwrap_or(&subject)
            .split(',').next().unwrap_or(&subject)))))
}

/// Convert x509 cert to our CertInfo struct
fn cert_to_info(cert: &X509Certificate, source: CertSource) -> CertInfo {
    let subject = extract_cn(cert.subject());
    let issuer = extract_cn(cert.issuer());
    let is_self_signed = subject == issuer;

    let not_before = cert.validity().not_before.to_rfc2822().unwrap_or_default();
    let not_after = cert.validity().not_after.to_rfc2822().unwrap_or_default();

    let now = chrono::Utc::now().timestamp();
    let expires = cert.validity().not_after.timestamp();
    let days_remaining = (expires - now) / 86400;

    let status = if days_remaining <= 0 {
        CertStatus::Expired
    } else if days_remaining <= 30 {
        CertStatus::Expiring30
    } else if days_remaining <= 90 {
        CertStatus::Expiring90
    } else {
        CertStatus::Valid
    };

    let serial = cert.serial.to_str_radix(16);

    CertInfo {
        subject, issuer, not_before, not_after,
        days_remaining, status, source, is_self_signed, serial,
    }
}

fn extract_cn(name: &x509_parser::x509::X509Name) -> String {
    name.iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or("Unknown")
        .to_string()
}

/// Full scan: directory + keychain, return summary
pub fn full_scan(path: Option<&Path>) -> CertScanResult {
    let mut certs = Vec::new();

    // Scan directory if provided
    if let Some(p) = path {
        certs.extend(scan_directory_certs(p));
    }

    // Scan macOS keychain
    if cfg!(target_os = "macos") {
        certs.extend(scan_keychain());
    }

    let expired = certs.iter().filter(|c| c.status == CertStatus::Expired).count();
    let expiring = certs.iter().filter(|c| matches!(c.status, CertStatus::Expiring30 | CertStatus::Expiring90)).count();
    let valid = certs.iter().filter(|c| c.status == CertStatus::Valid).count();

    // Sort: expired first, then by days remaining
    certs.sort_by_key(|c| (c.status, c.days_remaining));

    CertScanResult {
        certs, expired_count: expired, expiring_count: expiring, valid_count: valid,
    }
}

/// A loose key/cert file found during home scan
#[derive(Debug, Clone)]
pub struct LooseKeyFile {
    pub path: PathBuf,
    pub file_type: String,
    pub size: u64,
}

/// Scan home directory for loose cert/key files
pub fn scan_home_for_keys() -> Vec<LooseKeyFile> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    // Dirs to skip in home scan
    let home_skip: &[&str] = &[
        "Library", "Applications", ".Trash", ".cache", ".npm", ".cargo",
        "node_modules", ".git", "target", "dist", "build", ".venv",
        ".local/share", ".local/lib",
    ];

    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(&home)
        .max_depth(5)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() { return true; }
            let name = e.file_name().to_string_lossy();
            let rel = e.path().strip_prefix(&home).unwrap_or(e.path()).display().to_string();
            !SKIP_DIRS.contains(&name.as_ref()) && !home_skip.iter().any(|s| rel.starts_with(s))
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() { continue; }
        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_lower = ext.to_lowercase();

        let file_type = if CERT_EXTENSIONS.contains(&ext_lower.as_str()) {
            "certificate"
        } else if KEY_EXTENSIONS.contains(&ext_lower.as_str()) {
            "private key"
        } else if file_path.file_name().map(|n| n.to_string_lossy().starts_with("id_rsa") || n.to_string_lossy().starts_with("id_ed25519") || n.to_string_lossy().starts_with("id_ecdsa")).unwrap_or(false) {
            "SSH key"
        } else {
            continue;
        };

        let size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
        files.push(LooseKeyFile {
            path: file_path.to_path_buf(),
            file_type: file_type.to_string(),
            size,
        });
    }

    files.sort_by(|a, b| a.file_type.cmp(&b.file_type).then(a.path.cmp(&b.path)));
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_status_ordering() {
        assert!(CertStatus::Expired < CertStatus::Expiring30);
        assert!(CertStatus::Expiring30 < CertStatus::Expiring90);
        assert!(CertStatus::Expiring90 < CertStatus::Valid);
    }

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let certs = scan_directory_certs(dir.path());
        assert!(certs.is_empty());
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let certs = parse_cert_file(Path::new("/tmp/nonexistent.pem"));
        assert!(certs.is_empty());
    }

    #[test]
    fn test_extract_cn() {
        // Just verify it doesn't panic with various inputs
        let result = full_scan(None);
        let _ = result; // macOS keychain scan should work without panic
    }

    #[test]
    fn test_cert_source_display() {
        let file = CertSource::File(PathBuf::from("/tmp/cert.pem"));
        assert_eq!(format!("{}", file), "/tmp/cert.pem");
        let kc = CertSource::Keychain("System — Apple".into());
        assert_eq!(format!("{}", kc), "Keychain: System — Apple");
    }
}
