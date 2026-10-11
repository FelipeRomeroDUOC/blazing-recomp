# scripts/diagnostico/compilar_recomp_aot.ps1
# Arnés de automatización para invocar recomp build preservando el código C11 nativo
# y actualizando automáticamente src-c/ con las nuevas unidades generadas.

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

# 1. Localizar binario recomp
$recompBin = $null
$possiblePaths = @(
    "recomp",
    (Join-Path $RepoRoot "..\gba-recomp\gba-recomp-launcher-0.4-x86_64-pc-windows-msvc\recomp.exe"),
    (Join-Path $RepoRoot "..\gba-recomp\target\release\recomp.exe"),
    (Join-Path $RepoRoot "..\gba-recomp\target\debug\recomp.exe")
)

foreach ($path in $possiblePaths) {
    if (Get-Command $path -ErrorAction SilentlyContinue) {
        $recompBin = $path
        break
    } elseif (Test-Path $path) {
        $recompBin = (Resolve-Path $path).Path
        break
    }
}

if (-not $recompBin) {
    Write-Error "No se encontro el ejecutable 'recomp'. Verifique su instalacion en PATH o en gba-recomp hermano."
    exit 1
}

Write-Host "==> Usando recompilador: $recompBin"

# 2. Localizar ROM y BIOS
$romPath = Join-Path $RepoRoot "roms\1575 - Fire Emblem (E)(Independent).gba"
$biosPath = Join-Path $RepoRoot "roms\gba_bios.bin"
$labelsPath = Join-Path $RepoRoot "blazing.labels.toml"

if (-not (Test-Path $romPath)) {
    Write-Error "No se encontro la ROM oficial en $romPath."
    exit 1
}

if (-not (Test-Path $biosPath)) {
    Write-Error "No se encontro la BIOS oficial en $biosPath."
    exit 1
}

# 3. Invocar recomp build preservando archivos C intermedios
Write-Host "==> Iniciando recompilacion AOT C11 (RECOMP_KEEP_C=1)..."
$env:RECOMP_KEEP_C = "1"

Push-Location $RepoRoot
try {
    & $recompBin build $romPath --bios $biosPath --labels $labelsPath --ram
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Fallo recomp build con codigo $LASTEXITCODE."
        exit $LASTEXITCODE
    }

    # 4. Copiar archivos .c actualizados a src-c/
    $outDir = Join-Path $RepoRoot "out"
    $srcCDir = Join-Path $RepoRoot "src-c"

    if (Test-Path (Join-Path $outDir "*.c")) {
        Copy-Item (Join-Path $outDir "*.c") $srcCDir -Force
        Write-Host "==> Codigo C11 nativo sincronizado en src-c/ exitosamente."
    } else {
        Write-Warning "No se encontraron archivos .c en out/. Verifique la ejecucion de recomp."
    }
} finally {
    Pop-Location
}
