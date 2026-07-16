<div align="center">

# fpm (Fast Python Manager)

Un gestor de versiones de Python rapido para Windows, construido en Rust.
Envuelve el Python Install Manager oficial (`py`/`pymanager`) para cambio de
version de Python por sesion, inspirado en [fnm](https://github.com/Schniz/fnm).

[![Crates.io](https://img.shields.io/crates/v/fpm?style=flat-square)](https://crates.io/crates/fpm)
[![npm](https://img.shields.io/npm/v/fpm?style=flat-square)](https://www.npmjs.com/package/fpm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](../LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/creativelaides/fpm/ci.yml?style=flat-square&label=CI)](https://github.com/creativelaides/fpm/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/creativelaides/fpm?style=flat-square&label=Release)](https://github.com/creativelaides/fpm/releases/latest)

<br>

<div style="display: inline-block; border-radius: 50%; overflow: hidden; width: 125px; height: 125px;">
  <img src="../assets/kwak_logo_sponsor.jpg" width="125" height="125" alt="KWAK — Kit for Windows Application Kickstart" />
</div>

<sub>Parte de <strong>KWAK</strong> — <em>Kit for Windows Application Kickstart</em></sub>

</div>

## Funciones

- **Nativo de Windows**: Construido para Windows con cambio de shim basado en
  junctions NTFS.
- **Cambio por sesion**: Cambia versiones de Python por sesion de shell sin
  modificar el default global.
- **Soporte de archivos de version**: Lee `.python-version` y `pyproject.toml`
  (`requires-python` / dependencia `python` de Poetry) con matching de
  especificadores PEP 440.
- **Pass-through a `py`**: Cualquier comando no reconocido se reenvia verbatim
  a `py.exe`, por lo que los aliases y flujos existentes siguen funcionando.
- **Integracion con shell**: Genera un script de PowerShell que configura
  `FPM_DIR`, `FPM_MULTISHELL_PATH` y PATH — con conmutacion automatica
  opcional mediante `use-on-cd`.

## Instalacion

### Requisitos previos

- **PyManager 26.x+** instalado en Windows (`py.exe` en PATH). Descarga desde
  [python.org](https://www.python.org/downloads/) o ejecuta
  `winget install Python.Python.3` (el launcher oficial viene con las
  instalaciones de Python).

### Compilar desde el codigo fuente

```sh
git clone https://github.com/creativelaides/fpm.git
cd fpm
cargo build --release
```

Agrega el directorio `target/release` a tu PATH:

```powershell
# Agregar al perfil de PowerShell (ver Configuracion de Shell abajo para la ruta del perfil)
$env:PATH += ";$PWD\target\release"
```

> **Usando cargo install (futuro)**: Una vez publicado en crates.io, podras
> ejecutar `cargo install fpm` para instalar el binario directamente.

## Configuracion de Shell

### PowerShell

Agrega lo siguiente al final de tu archivo de perfil de PowerShell:

```powershell
fpm env --use-on-cd --shell powershell | Out-String | Invoke-Expression
```

Esto evalua el script `fpm env` cada vez que se inicia un nuevo shell,
configurando `FPM_DIR`, anteponiendo el directorio shim de la sesion a PATH,
e instalando el hook de `Set-Location` para cambio automatico de version al
navegar directorios con `cd`.

#### Ubicaciones del perfil

| Version de shell     | Ruta del perfil                                                                   |
| --------------------- | --------------------------------------------------------------------------------- |
| PowerShell 6+        | `%userprofile%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`             |
| Windows PowerShell   | `%userprofile%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`     |

Para crear el perfil si no existe:

```powershell
if (-not (Test-Path $profile)) { New-Item $profile -Force }
```

Para editar el perfil:

```powershell
Invoke-Item $profile
```

#### Sin use-on-cd

Si prefieres cambio manual (sin hook automatico de `cd`), omite `--use-on-cd`:

```powershell
fpm env --shell powershell | Out-String | Invoke-Expression
```

## Uso

### Comandos

| Comando                      | Descripcion                                                        |
| ---------------------------- | ------------------------------------------------------------------ |
| `fpm use [version]`          | Cambia a una version de Python para esta sesion. Resuelve desde    |
|                              | `.python-version` o `pyproject.toml` si no se especifica version.  |
| `fpm list`                   | Lista los runtimes de Python instalados.                           |
| `fpm current`                | Imprime la version de Python actualmente activa.                   |
| `fpm default [tag]`          | Lee o establece la version default de Python (escribe              |
|                              | `pymanager.json`).                                                 |
| `fpm env --shell powershell` | Emite un script de integracion de shell. Usa `--use-on-cd` para    |
|                              | conmutacion automatica al cambiar de directorio.                   |
| `fpm install <tag>`          | Instala una version de Python via `py install <tag>`.              |

### Ejemplos

```sh
# Listar versiones de Python instaladas
fpm list

# Cambiar a Python 3.14 para esta sesion
fpm use 3.14

# Imprimir la version activa
fpm current

# Establecer 3.13 como default (persiste entre sesiones via pymanager.json)
fpm default 3.13

# Instalar una nueva version de Python
fpm install 3.12

# Pass-through a py.exe — todos los args no reconocidos se reenvian verbatim
fpm -m http.server 8000
fpm script.py
```

### Resolucion de archivo de version

`fpm use` sin argumentos recorre hacia arriba desde el directorio actual
buscando:

1. `.python-version` — contiene un tag de version (ej. `3.14` o `3.14-64`).
2. `pyproject.toml` — `[project] requires-python` (PEP 621) o
   `[tool.poetry.dependencies] python` con un especificador PEP 440 (ej.
   `>=3.12`, `~=3.13.0`, `==3.14.*`).

La primera coincidencia gana. Para especificadores, fpm reduce contra la lista
de runtimes instalados y selecciona la version mas alta que coincida.

```sh
# Crear un archivo .python-version
echo "3.14" > .python-version

# Ahora `fpm use` (sin args) cambia a 3.14
fpm use
```

## Configuracion

### FPM_DIR

El directorio de datos de fpm. Por defecto `%LocalAppData%\fpm`. Sobreescribe
con la variable de entorno `FPM_DIR`:

```powershell
$env:FPM_DIR = "D:\mis-datos-fpm"
```

Los directorios shim de sesion se crean bajo `FPM_DIR/multishells/<session-id>/`.

### PYTHON_MANAGER_DEFAULT

Establecida en proceso por `fpm use` para marcar la version activa de la
sesion actual. Leida por `fpm current` como fuente principal de verdad.

### pymanager.json

Ubicado en `%AppData%\Python\pymanager.json`. Gestionado por `fpm default` (y
por PyManager mismo). `fpm use` **no** escribe en este archivo — el cambio es
solo de sesion.

## Ciclo de vida del directorio de sesion

Cada invocacion de `fpm env` crea un directorio de sesion unico bajo
`FPM_DIR/multishells/<pid>_<random>/`. El script de PowerShell generado
registra un evento de engine `PowerShell.Exiting` que elimina best-effort el
directorio de sesion al cerrar el shell limpiamente.

Los directorios obsoletos de shells que crashearon no rompen otras sesiones —
el ID de sesion unico previene colisiones. Puedes limpiar manualmente los
directorios obsoletos de forma segura:

```powershell
Remove-Item -Recurse -Force "$env:FPM_DIR\multishells\*"
```

## Contribuir

Consulta [CONTRIBUTING.md](../CONTRIBUTING.md) para la configuracion de
desarrollo, convenciones de commits y detalles de CI.

Para commits convencionales interactivos:

```sh
pnpm cz
```

Para changesets (bumps de version y changelog):

```sh
pnpm changeset
```

## Licencia

MIT