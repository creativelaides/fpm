use crate::services::pymanager::Runtime;
use crate::services::remote::RemoteVersion;
use std::path::Path;

pub fn print_detailed_version(
    cli_version: &str,
    launcher_version: &str,
    python_version: &str,
) -> String {
    format!(
        "fpm {}\n\n{}\nActive Python: {}",
        cli_version,
        launcher_version.trim(),
        python_version.trim()
    )
}

pub fn print_remote_versions(
    versions: &[RemoteVersion],
    show_pre: bool,
    offline_fallback: bool,
) -> String {
    let mut out = String::new();
    if offline_fallback {
        out.push_str("Warning: network error. Showing cached versions.\n\n");
    }

    let filtered: Vec<&RemoteVersion> = if show_pre {
        versions.iter().collect()
    } else {
        versions
            .iter()
            .filter(|v| !v.version.chars().any(|c| c.is_alphabetic()))
            .collect()
    };

    if filtered.is_empty() {
        out.push_str("No remote versions found.");
        return out;
    }

    let version_w = filtered
        .iter()
        .map(|v| v.version.len())
        .max()
        .unwrap_or(7)
        .max(7);
    out.push_str(&format!(
        "{:<width_v$}  RELEASE DATE\n",
        "VERSION",
        width_v = version_w
    ));
    out.push_str(&format!(
        "{:-<width_v$}  ------------\n",
        "",
        width_v = version_w
    ));

    for v in filtered {
        let date = v.release_date.as_deref().unwrap_or("unknown");
        out.push_str(&format!(
            "{:<width_v$}  {}\n",
            v.version,
            date,
            width_v = version_w
        ));
    }

    out
}

pub fn format_local_runtimes(runtimes: &[Runtime]) -> String {
    if runtimes.is_empty() {
        return "No local runtimes found.".to_string();
    }
    let version_w = runtimes
        .iter()
        .map(|r| r.version.len())
        .max()
        .unwrap_or(7)
        .max(7);
    let tag_w = runtimes
        .iter()
        .map(|r| r.tag.len())
        .max()
        .unwrap_or(3)
        .max(3);

    let mut out = String::new();
    out.push_str(&format!(
        "{:<width_v$}  {:<width_t$}  PATH\n",
        "VERSION",
        "TAG",
        width_v = version_w,
        width_t = tag_w
    ));
    out.push_str(&format!(
        "{:-<width_v$}  {:-<width_t$}  ----\n",
        "",
        "",
        width_v = version_w,
        width_t = tag_w
    ));

    for rt in runtimes {
        let marker = if rt.is_default { "* " } else { "  " };
        out.push_str(&format!(
            "{}{:<width_v$}  {:<width_t$}  {}\n",
            marker,
            rt.version,
            rt.tag,
            rt.executable.display(),
            width_v = version_w,
            width_t = tag_w
        ));
    }
    out
}

pub fn format_current_version(active_tag: Option<&str>, py_version_line: Option<&str>) -> String {
    match (active_tag, py_version_line) {
        (Some(tag), Some(line)) => format!("{} (tag: {})", line, tag),
        (None, Some(line)) => line.to_string(),
        (Some(tag), None) => format!("Python {} (configured, py -V unavailable)", tag),
        (None, None) => "No default Python configured.".to_string(),
    }
}

pub fn format_default_read(default_tag: Option<&str>) -> String {
    match default_tag {
        Some(tag) => tag.to_string(),
        None => "No default Python configured.".to_string(),
    }
}

pub fn format_default_unset(removed: bool) -> String {
    if removed {
        "Default Python unset.".to_string()
    } else {
        "No default was configured.".to_string()
    }
}

pub fn format_default_dry_run(tag: &str, version: &str, install_dir: &Path) -> String {
    format!(
        "Would set default to {} and activate Python {} at {}",
        tag,
        version,
        install_dir.display()
    )
}

pub fn format_default_set_success(tag: &str) -> String {
    format!("Default Python set to {}. Using Python {}.", tag, tag)
}

pub fn format_use_success(tag: &str) -> String {
    format!("Using Python {}", tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_detailed_version() {
        let out = print_detailed_version(
            "1.0.0",
            "Python Launcher for Windows Version 3.14.6",
            "Python 3.14.6",
        );
        assert_eq!(
            out,
            "fpm 1.0.0\n\nPython Launcher for Windows Version 3.14.6\nActive Python: Python 3.14.6"
        );
    }

    #[test]
    fn test_print_remote_versions() {
        let versions = vec![
            RemoteVersion {
                version: "3.10.0".to_string(),
                release_date: None,
            },
            RemoteVersion {
                version: "3.11.0a1".to_string(),
                release_date: None,
            },
        ];

        let output = print_remote_versions(&versions, true, false);
        assert!(output.contains("3.10.0"));
        assert!(output.contains("3.11.0a1"));

        let output2 = print_remote_versions(&versions, false, false);
        assert!(output2.contains("3.10.0"));
        assert!(!output2.contains("3.11.0a1"));

        let output3 = print_remote_versions(&versions, true, true);
        assert!(output3.contains("Warning: network error"));
    }

    #[test]
    fn test_format_local_runtimes() {
        let output = format_local_runtimes(&[]);
        assert_eq!(output, "No local runtimes found.");
    }

    #[test]
    fn test_format_current_version() {
        assert_eq!(
            format_current_version(Some("3.12"), Some("Python 3.12.0")),
            "Python 3.12.0 (tag: 3.12)"
        );
        assert_eq!(
            format_current_version(None, Some("Python 3.11.0")),
            "Python 3.11.0"
        );
        assert_eq!(
            format_current_version(Some("3.10"), None),
            "Python 3.10 (configured, py -V unavailable)"
        );
        assert_eq!(
            format_current_version(None, None),
            "No default Python configured."
        );
    }

    #[test]
    fn test_format_default_read() {
        assert_eq!(format_default_read(Some("3.12")), "3.12");
        assert_eq!(format_default_read(None), "No default Python configured.");
    }

    #[test]
    fn test_format_default_unset() {
        assert_eq!(format_default_unset(true), "Default Python unset.");
        assert_eq!(format_default_unset(false), "No default was configured.");
    }

    #[test]
    fn test_format_default_dry_run() {
        let p = Path::new("/fake/path");
        let out = format_default_dry_run("3.12", "3.12.0", p);
        assert_eq!(
            out,
            format!(
                "Would set default to 3.12 and activate Python 3.12.0 at {}",
                p.display()
            )
        );
    }

    #[test]
    fn test_format_default_set_success() {
        assert_eq!(
            format_default_set_success("3.12"),
            "Default Python set to 3.12. Using Python 3.12."
        );
    }

    #[test]
    fn test_format_use_success() {
        assert_eq!(format_use_success("3.12"), "Using Python 3.12");
    }
}
