# PyManager CI Installation Strategy

> **Always consult**: https://docs.python.org/3/using/windows.html
> **Troubleshooting section**: https://docs.python.org/3/using/windows.html#pymanager-troubleshoot

## Problem

GitHub Actions Windows runners ship with a **legacy `py.exe` launcher** at `C:\Windows\py.exe` that does NOT support `py list`, `py install`, or any PyManager subcommands. It only supports `py -V:<tag>` for launching installed runtimes.

The python.org Python installer (`python-3.14.6-amd64.exe`) also only installs the **legacy** launcher — it does NOT install the new PyManager.

The new Python Install Manager (PyManager) is distributed as:
- **MSIX** (via Microsoft Store or python.org/ftp/python/pymanager/)
- **MSI** (for environments without MSIX support, e.g. Windows Server 2019)

## Solution

### Step 1: Remove the legacy launcher

```powershell
Remove-Item "C:\Windows\py.exe" -Force -ErrorAction SilentlyContinue
Remove-Item "C:\Windows\pyshellext.dll" -Force -ErrorAction SilentlyContinue
```

### Step 2: Install PyManager per Windows version

| Windows version | Method | Command |
|----------------|--------|---------|
| Server 2022 (windows-latest) | MSIX via AppInstaller | `Add-AppxPackage -AppInstallerFile "https://www.python.org/ftp/python/pymanager/pymanager.appinstaller"` |
| Server 2019 (windows-2019) | MSI via msiexec | `msiexec /i python-manager-26.3.msi /quiet /norestart` |

MSI download URL: `https://www.python.org/ftp/python/pymanager/python-manager-26.3.msi`

### Step 3: Install Python runtimes

```powershell
py install 3.14
py install 3.13
py list
```

## Key facts

- The docs explicitly state: "Windows Server 2019 is the only version of Windows that CPython supports that does not support MSIX. For Windows Server 2019, you should use the MSI."
- The MSI installs to Program Files and modifies system PATH.
- The MSIX installs as an app execution alias in `%LocalAppData%\Microsoft\WindowsApps\`.
- PyManager versions are listed at: https://www.python.org/ftp/python/pymanager/
- Latest stable as of 2026-07-16: `python-manager-26.3.msi` / `.msix`

## CI workflow

File: `.github/workflows/windows-compat.yml`

- Jobs run in **parallel** (`fail-fast: false`) — Server 2022 and Server 2019 do not cancel each other.
- Matrix uses `pymanager-method: msix` or `pymanager-method: msi` to select the install strategy.
- After PyManager install, both runners run the same test suite: unit tests + `#[ignore]` integration tests + smoke tests.

## Reference

- Python Install Manager docs: https://docs.python.org/3/using/windows.html#python-install-manager
- Advanced installation: https://docs.python.org/3/using/windows.html#advanced-installation
- Troubleshooting: https://docs.python.org/3/using/windows.html#pymanager-troubleshoot
- PyManager releases: https://www.python.org/ftp/python/pymanager/