// Shell integration trait.
//
// Spec: powershell-shell-integration
// Design: Data Flow for `fpm env --shell powershell`
//
// Each shell backend renders a script that the user evaluates in their shell
// to integrate fpm: it sets env vars, prepends the session shim dir to PATH,
// and optionally installs a use-on-cd hook. The trait keeps the door open for
// cmd.exe or bash backends in the future without changing the `env` command.

use std::path::Path;

/// A shell backend that can render an fpm integration script.
///
/// `render` returns the complete shell script printed to stdout by
/// `fpm env --shell <name>`. The user pipes/evaluates it in their shell.
pub trait Shell {
    /// Render the integration script for this shell.
    ///
    /// `session_dir` is the per-session multishell directory (the junction
    /// that will be retargeted by `fpm use`). `use_on_cd` controls whether
    /// automatic version-switching on directory change is emitted.
    fn render(&self, session_dir: &Path, use_on_cd: bool) -> String;
}

pub mod powershell;
