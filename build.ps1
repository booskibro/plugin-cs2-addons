<#
.SYNOPSIS
    Local (non-CI) build of the CS2 Addons plugin on Windows.

.DESCRIPTION
    Does what .github/workflows/build.yml does, minus the Ubuntu runner and
    `make` (not present on this machine):

      1. builds ..\gameap-api\web\plugin-sdk - the frontend's `file:` dependency
      2. builds the Vue frontend into frontend\dist (embedded by build.rs)
      3. builds the wasm and copies it to cs2-addons.wasm

    Node is installed at C:\Program Files\nodejs but is not on PATH, so it is
    prepended for the duration of this script only - nothing global changes.
    Rust comes from rustup (.cargo\bin, already on the user PATH); the pinned
    toolchain and wasm32-wasip1 target come from rust-toolchain.toml.

.PARAMETER Test
    Run both test suites (cargo test + vitest) before building the wasm.

.PARAMETER SkipFrontend
    Reuse the existing frontend\dist and build only the wasm.

.EXAMPLE
    .\build.ps1
.EXAMPLE
    .\build.ps1 -Test
#>
param(
    [switch]$Test,
    [switch]$SkipFrontend
)

$ErrorActionPreference = 'Stop'

$root     = $PSScriptRoot
$frontend = Join-Path $root 'frontend'
$sdk      = Join-Path (Split-Path $root -Parent) 'gameap-api\web\plugin-sdk'
$wasmOut  = Join-Path $root 'target\wasm32-wasip1\release\cs2_addons.wasm'
$artifact = Join-Path $root 'cs2-addons.wasm'

foreach ($dir in @('C:\Program Files\nodejs', (Join-Path $env:USERPROFILE '.cargo\bin'))) {
    if (Test-Path $dir) { $env:PATH = "$dir;$env:PATH" }
}

function Invoke-Step {
    param([string]$Label, [string]$WorkDir, [scriptblock]$Body)

    Write-Host ""
    Write-Host "==> $Label" -ForegroundColor Cyan
    Push-Location $WorkDir
    try {
        # Native tools (cargo, npm) write progress to stderr, which PowerShell 5.1
        # turns into terminating ErrorRecords when the caller pipes with 2>&1.
        # Failure is detected from $LASTEXITCODE below, so don't stop on those.
        $ErrorActionPreference = 'Continue'
        & $Body
        if ($LASTEXITCODE -ne 0) { throw "$Label failed (exit code $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

# --- preflight ---------------------------------------------------------------

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "node not found. Install Node.js 22+ (expected at C:\Program Files\nodejs)."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo not found. Install rustup: winget install --id Rustlang.Rustup --exact"
}
if (-not (Test-Path $sdk)) {
    throw @"
The gameap panel checkout is missing: $sdk
The frontend depends on it as file:../../gameap-api/web/plugin-sdk. Create it with:
    git clone --depth 1 https://github.com/gameap/gameap.git "$(Split-Path $root -Parent)\gameap-api"
"@
}

# --- 1. plugin SDK -----------------------------------------------------------
# Built with vite directly: `npm run build` also runs tsc, which fails on
# @gameap/ui's missing type declarations. The frontend only needs dist\index.js.
# The @gameap/ui tarball on npm ships without its icons\ directory, so the
# package from the gameap checkout is installed over it.

if (-not $SkipFrontend) {
    if (-not (Test-Path (Join-Path $sdk 'dist\index.js'))) {
        Invoke-Step 'Building plugin SDK' $sdk {
            npm ci
            npm install --no-save ..\frontend\packages\gameap-ui
            npx vite build
        }
    } else {
        Write-Host "==> Plugin SDK already built (delete its dist\ to rebuild)" -ForegroundColor DarkGray
    }
}

# --- 2. frontend -------------------------------------------------------------

if (-not $SkipFrontend) {
    Invoke-Step 'Building frontend' $frontend {
        if (-not (Test-Path 'node_modules')) { npm ci }
        npm run build
    }
}

# --- 3. tests ----------------------------------------------------------------

if ($Test) {
    Invoke-Step 'Testing frontend' $frontend { npm test }
    Invoke-Step 'Testing backend'  $root     { cargo test }
}

# --- 4. wasm -----------------------------------------------------------------

Invoke-Step 'Building wasm' $root { cargo build --target wasm32-wasip1 --release }

Copy-Item $wasmOut $artifact -Force

$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"(.+)"').Matches[0].Groups[1].Value
$size    = [math]::Round((Get-Item $artifact).Length / 1KB)

Write-Host ""
Write-Host "Built cs2-addons.wasm  v$version  ($size KB)" -ForegroundColor Green
