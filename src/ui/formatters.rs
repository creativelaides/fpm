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
}
