//! Certificate scanner CLI command.

use std::path::PathBuf;
use crate::scanner::cert_scanner::{self, CertStatus, CertSource};
use super::shorten_path;

pub fn cmd_certs(path: Option<PathBuf>, keychain_only: bool, expired_only: bool, summary_only: bool) {
    println!("  SPARK Certificate Scanner\n");

    let scan_path = path.as_deref();

    let result = if keychain_only {
        println!("  Scanning macOS Keychain...\n");
        let certs = cert_scanner::scan_keychain();
        let expired = certs.iter().filter(|c| c.status == CertStatus::Expired).count();
        let expiring = certs.iter().filter(|c| matches!(c.status, CertStatus::Expiring30 | CertStatus::Expiring90)).count();
        let valid = certs.iter().filter(|c| c.status == CertStatus::Valid).count();
        cert_scanner::CertScanResult { certs, expired_count: expired, expiring_count: expiring, valid_count: valid }
    } else {
        let p = scan_path.unwrap_or_else(|| {
            Box::leak(Box::new(std::env::current_dir().unwrap_or_default()))
        });
        println!("  Scanning {} + system Keychain...\n", shorten_path(&p.display().to_string()));
        cert_scanner::full_scan(Some(p))
    };

    if result.certs.is_empty() {
        println!("  No certificates found.");
        return;
    }

    // Summary table by year
    if !summary_only {
        print_certs(&result.certs, expired_only);
    }

    // Home scan for loose key/cert files
    {
        eprint!("  Scanning ~/ for loose key/cert files");
        let loose = cert_scanner::scan_home_for_keys();
        eprintln!(".. {} found\n", loose.len());

        if !loose.is_empty() {
            println!("  \x1b[33m--- Loose Key/Cert Files in ~/ ---\x1b[0m\n");

            // Group by type
            let mut by_type: std::collections::BTreeMap<&str, Vec<&cert_scanner::LooseKeyFile>> = std::collections::BTreeMap::new();
            for f in &loose {
                by_type.entry(&f.file_type).or_default().push(f);
            }

            for (file_type, files) in &by_type {
                let (icon, color) = match *file_type {
                    "private key" | "SSH key" => ("!!", "31"),
                    _ => ("~ ", "33"),
                };
                println!("    \x1b[{}m{}\x1b[0m  \x1b[1m{}\x1b[0m \x1b[90m({} files)\x1b[0m",
                    color, icon, file_type, files.len());
                for f in files {
                    let short = shorten_path(&f.path.display().to_string());
                    let size = crate::utils::fs::format_size(f.size);
                    println!("        \x1b[90m{} ({})\x1b[0m", short, size);
                }
                println!();
            }

            println!("  \x1b[90mReview these files — private keys outside of ~/.ssh/ may be accidental.\x1b[0m\n");
        }
    }

    print_summary(&result);

    // Tip for cleanup
    if result.expired_count > 0 {
        println!("  \x1b[90mTo remove expired certs from Keychain:\x1b[0m");
        println!("  \x1b[90m  open /Applications/Utilities/Keychain\\ Access.app\x1b[0m");
        println!("  \x1b[90m  or: security delete-certificate -c \"cert-name\" login.keychain\x1b[0m\n");
    }
}

fn print_certs(certs: &[cert_scanner::CertInfo], expired_only: bool) {
    let expired: Vec<_> = certs.iter().filter(|c| c.status == CertStatus::Expired).collect();
    let expiring: Vec<_> = certs.iter().filter(|c| matches!(c.status, CertStatus::Expiring30 | CertStatus::Expiring90)).collect();
    let valid: Vec<_> = certs.iter().filter(|c| c.status == CertStatus::Valid).collect();

    if !expired.is_empty() {
        println!("  \x1b[31m--- Expired ({}) ---\x1b[0m\n", expired.len());
        print_grouped(&expired);
    }

    if expired_only { return; }

    if !expiring.is_empty() {
        println!("  \x1b[33m--- Expiring Soon ({}) ---\x1b[0m\n", expiring.len());
        print_grouped(&expiring);
    }

    if !valid.is_empty() {
        println!("  \x1b[32m--- Valid ({}) ---\x1b[0m\n", valid.len());
        print_grouped(&valid);
    }
}

/// Group certificates by issuer prefix to reduce repetition
fn print_grouped(certs: &[&cert_scanner::CertInfo]) {
    use std::collections::BTreeMap;

    // Group by issuer
    let mut by_issuer: BTreeMap<String, Vec<&&cert_scanner::CertInfo>> = BTreeMap::new();
    for cert in certs {
        by_issuer.entry(cert.issuer.clone()).or_default().push(cert);
    }

    for (issuer, group) in &by_issuer {
        let (icon, color) = match group[0].status {
            CertStatus::Expired => ("!!", "31"),
            CertStatus::Expiring30 => ("! ", "33"),
            CertStatus::Expiring90 => ("~ ", "33"),
            CertStatus::Valid => ("+ ", "32"),
        };

        if group.len() == 1 {
            let cert = group[0];
            let days_str = format_days(cert.days_remaining);
            let self_signed = if cert.is_self_signed { " (self-signed)" } else { "" };
            println!("    \x1b[{}m{}\x1b[0m  \x1b[1m{}\x1b[0m{}", color, icon, cert.subject, self_signed);
            println!("        \x1b[90missuer: {} | {} | {}\x1b[0m", issuer, cert.not_after, days_str);
            match &cert.source {
                CertSource::File(p) => println!("        \x1b[90mfile: {}\x1b[0m", shorten_path(&p.display().to_string())),
                CertSource::Keychain(name) => println!("        \x1b[90m{}\x1b[0m", name),
            }
            println!();
        } else {
            // Group header with count
            // Oldest and newest in the group
            let oldest = group.iter().min_by_key(|c| c.days_remaining).unwrap();
            let newest = group.iter().max_by_key(|c| c.days_remaining).unwrap();

            println!("    \x1b[{}m{}\x1b[0m  \x1b[1m{}\x1b[0m \x1b[90m({} certs)\x1b[0m",
                color, icon, issuer, group.len());

            if group.len() <= 5 {
                for cert in group {
                    let days_str = format_days(cert.days_remaining);
                    let self_signed = if cert.is_self_signed { " (ss)" } else { "" };
                    println!("        \x1b[90m-- {}{} | {} | {}\x1b[0m",
                        cert.subject, self_signed, cert.not_after, days_str);
                }
            } else {
                let others = group.len() - 2;
                println!("        \x1b[90moldest: {} | {}\x1b[0m",
                    oldest.not_after, format_days(oldest.days_remaining));
                println!("        \x1b[90m... {} other certs ...\x1b[0m", others);
                println!("        \x1b[90mnewest: {} | {}\x1b[0m",
                    newest.not_after, format_days(newest.days_remaining));
            }
            println!();
        }
    }
}

fn format_days(days: i64) -> String {
    if days <= 0 {
        format!("expired {}d ago", -days)
    } else {
        format!("{}d remaining", days)
    }
}

fn print_summary(result: &cert_scanner::CertScanResult) {
    // Group expired by age buckets
    let mut by_year: std::collections::BTreeMap<i32, (usize, usize, usize)> = std::collections::BTreeMap::new();
    for cert in &result.certs {
        let year = if cert.days_remaining <= 0 {
            // Group by how long ago it expired
            let years_ago = (-cert.days_remaining / 365) as i32;
            -(years_ago + 1) // negative = expired years ago
        } else {
            0 // valid/expiring
        };
        let entry = by_year.entry(year).or_insert((0, 0, 0));
        match cert.status {
            CertStatus::Expired => entry.0 += 1,
            CertStatus::Expiring30 | CertStatus::Expiring90 => entry.1 += 1,
            CertStatus::Valid => entry.2 += 1,
        }
    }

    println!("\n  =================================");
    println!("  Certificate Summary\n");
    println!("  \x1b[31m{} expired\x1b[0m   \x1b[33m{} expiring\x1b[0m   \x1b[32m{} valid\x1b[0m   ({} total)\n",
        result.expired_count, result.expiring_count, result.valid_count, result.certs.len());

    if result.expired_count > 0 {
        println!("  \x1b[90mExpired by age:\x1b[0m");
        for (year, (expired, _, _)) in by_year.iter().rev() {
            if *expired == 0 { continue; }
            if *year == 0 { continue; }
            let age = -year;
            if age == 1 {
                println!("    \x1b[90m< 1 year:    {} certs\x1b[0m", expired);
            } else {
                println!("    \x1b[90m{}-{} years:  {} certs\x1b[0m", age - 1, age, expired);
            }
        }
        println!();
    }
}
