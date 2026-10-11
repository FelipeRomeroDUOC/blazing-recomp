# scripts/package_apk.ps1
# Arnés de empaquetado y firmado de APK para Blazing Recomp en Android.
# Cumple con la regla de portabilidad (sin rutas absolutas locales fijadas).

param (
    [string]$SdkPath = "",
    [string]$TargetAbi = "arm64-v8a",
    [string]$TargetArch = "",
    [switch]$SkipBuild = $false,
    [switch]$AllAbis = $true,
    [switch]$NoStrip = $false,
    [switch]$Debug = $false
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

# Si no se especifica TargetArch, mapear automáticamente desde TargetAbi
if (-not $TargetArch) {
    switch ($TargetAbi) {
        "arm64-v8a"   { $TargetArch = "aarch64-linux-android" }
        "x86_64"      { $TargetArch = "x86_64-linux-android" }
        "armeabi-v7a" { $TargetArch = "armv7-linux-androideabi" }
        "x86"         { $TargetArch = "i686-linux-android" }
        default       { $TargetArch = "aarch64-linux-android" }
    }
}

# 1. Detección portable de Android SDK
if (-not $SdkPath) {
    if ($env:ANDROID_HOME -and (Test-Path $env:ANDROID_HOME)) {
        $SdkPath = $env:ANDROID_HOME
    } elseif ($env:ANDROID_SDK_ROOT -and (Test-Path $env:ANDROID_SDK_ROOT)) {
        $SdkPath = $env:ANDROID_SDK_ROOT
    } elseif ($env:LOCALAPPDATA -and (Test-Path (Join-Path $env:LOCALAPPDATA "Android\Sdk"))) {
        $SdkPath = Join-Path $env:LOCALAPPDATA "Android\Sdk"
    } else {
        Write-Error "Android SDK no encontrado. Configure ANDROID_HOME o pase el parámetro -SdkPath."
        exit 1
    }
}

Write-Host "==> Android SDK: $SdkPath"

# 1.1 Detección de Android NDK (para cargo ndk, llvm-strip y clang AOT)
$llvmStrip = $null
$clang = $null
if (-not $env:ANDROID_NDK_HOME -and -not $env:NDK_HOME) {
    $ndkBase = Join-Path $SdkPath "ndk"
    if (Test-Path $ndkBase) {
        $ndkDirs = Get-ChildItem $ndkBase -Directory | Sort-Object {
            try { [version]$_.Name } catch { $_.Name }
        } -Descending
        if ($ndkDirs.Count -gt 0) {
            $env:ANDROID_NDK_HOME = $ndkDirs[0].FullName
            Write-Host "==> Android NDK: $env:ANDROID_NDK_HOME"
        }
    }
}

if ($env:ANDROID_NDK_HOME) {
    $potentialStrip = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-strip.exe"
    if (Test-Path $potentialStrip) {
        $llvmStrip = $potentialStrip
        Write-Host "==> LLVM Strip: $llvmStrip"
    }
    $potentialClang = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe"
    if (Test-Path $potentialClang) {
        $clang = $potentialClang
        Write-Host "==> Clang NDK: $clang"
    }
}

# 2. Detección de build-tools (aapt2, zipalign, apksigner) con orden numérico de versión
$buildToolsBase = Join-Path $SdkPath "build-tools"
if (-not (Test-Path $buildToolsBase)) {
    Write-Error "No se encontro el directorio build-tools en $buildToolsBase."
    exit 1
}

$buildToolsDirs = Get-ChildItem $buildToolsBase -Directory | Sort-Object {
    try { [version]$_.Name } catch { $_.Name }
} -Descending
if (-not $buildToolsDirs -or $buildToolsDirs.Count -eq 0) {
    Write-Error "No hay versiones de build-tools instaladas en $buildToolsBase."
    exit 1
}

$buildToolsDir = $buildToolsDirs[0].FullName
Write-Host "==> Build Tools: $buildToolsDir"

$aapt2 = Join-Path $buildToolsDir "aapt2.exe"
if (-not (Test-Path $aapt2)) { $aapt2 = Join-Path $buildToolsDir "aapt2" }
if (-not (Test-Path $aapt2)) {
    Write-Error "No se encontro aapt2 en $buildToolsDir."
    exit 1
}

$zipalign = Join-Path $buildToolsDir "zipalign.exe"
if (-not (Test-Path $zipalign)) { $zipalign = Join-Path $buildToolsDir "zipalign" }
if (-not (Test-Path $zipalign)) {
    Write-Error "No se encontro zipalign en $buildToolsDir."
    exit 1
}

$apksigner = Join-Path $buildToolsDir "apksigner.bat"
if (-not (Test-Path $apksigner)) { $apksigner = Join-Path $buildToolsDir "apksigner" }
if (-not (Test-Path $apksigner)) {
    Write-Error "No se encontro apksigner en $buildToolsDir."
    exit 1
}

# 3. Detección de android.jar con orden numérico de versión de plataforma
$platformJars = Get-ChildItem (Join-Path $SdkPath "platforms") -Filter "android.jar" -Recurse -File | Sort-Object {
    if ($_.Directory.Name -match 'android-(\d+)') {
        [int]$matches[1]
    } else {
        0
    }
} -Descending
if (-not $platformJars -or $platformJars.Count -eq 0) {
    Write-Error "No se encontro android.jar en platforms del SDK."
    exit 1
}
$androidJar = $platformJars[0].FullName
Write-Host "==> Plataforma android.jar: $androidJar"

# 4. Detección de Java / keytool / jar y configuración de entorno
$keytool = $null
$jar = $null

if ($env:JAVA_HOME) {
    $cKeytool = Join-Path $env:JAVA_HOME "bin\keytool.exe"
    if (-not (Test-Path $cKeytool)) { $cKeytool = Join-Path $env:JAVA_HOME "bin\keytool" }
    if (Test-Path $cKeytool) { $keytool = $cKeytool }

    $cJar = Join-Path $env:JAVA_HOME "bin\jar.exe"
    if (-not (Test-Path $cJar)) { $cJar = Join-Path $env:JAVA_HOME "bin\jar" }
    if (Test-Path $cJar) { $jar = $cJar }
}

if (-not $keytool -and (Get-Command keytool -ErrorAction SilentlyContinue)) {
    $keytool = (Get-Command keytool).Source
}
if (-not $jar -and (Get-Command jar -ErrorAction SilentlyContinue)) {
    $jar = (Get-Command jar).Source
}

if (-not $keytool -or -not $jar) {
    $potentialJbr = Join-Path $env:ProgramFiles "Android\Android Studio\jbr"
    if (Test-Path $potentialJbr) {
        if (-not $keytool) {
            $cKeytool = Join-Path $potentialJbr "bin\keytool.exe"
            if (Test-Path $cKeytool) { $keytool = $cKeytool }
        }
        if (-not $jar) {
            $cJar = Join-Path $potentialJbr "bin\jar.exe"
            if (Test-Path $cJar) { $jar = $cJar }
        }
        if (-not $env:JAVA_HOME) {
            $env:JAVA_HOME = $potentialJbr
        }
    }
}

if (-not $jar -and $keytool) {
    $keytoolDir = Split-Path $keytool
    $cJar = Join-Path $keytoolDir "jar.exe"
    if (-not (Test-Path $cJar)) { $cJar = Join-Path $keytoolDir "jar" }
    if (Test-Path $cJar) { $jar = $cJar }
}

# Detección de python para fallback de inyección STORED (sin compresión)
$python = $null
if (Get-Command python -ErrorAction SilentlyContinue) {
    $python = (Get-Command python).Source
} elseif ($env:ANDROID_NDK_HOME) {
    $ndkPy = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\python3\python.exe"
    if (Test-Path $ndkPy) { $python = $ndkPy }
}

if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME "bin"))) {
    $javaBin = Join-Path $env:JAVA_HOME "bin"
    if ($env:PATH -notlike "*$javaBin*") {
        $env:PATH = "$javaBin;$env:PATH"
    }
}

# Asegurar cargo en PATH si esta en ~/.cargo/bin
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path $cargoBin) {
        $env:PATH = "$cargoBin;$env:PATH"
    }
}

# 5. Keystore de depuracion
$keystoreDir = Join-Path $env:USERPROFILE ".android"
if (-not (Test-Path $keystoreDir)) {
    New-Item -ItemType Directory -Path $keystoreDir -Force | Out-Null
}
$debugKeystore = Join-Path $keystoreDir "debug.keystore"
if (-not (Test-Path $debugKeystore)) {
    Write-Host "==> Generando debug.keystore en $debugKeystore..."
    if (-not $keytool) {
        Write-Error "No se encontro keytool para generar el keystore de depuracion."
        exit 1
    }
    & $keytool -genkeypair -v -keystore $debugKeystore -storepass android -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Error generando debug.keystore con keytool."
        exit $LASTEXITCODE
    }
}

# 6. Compilar .so con cargo ndk y C11 AOT con Clang
$targetProfile = if ($Debug) { "debug" } else { "release" }

function Build-Recompiled-C {
    param(
        [string]$Abi
    )

    if (-not $clang) {
        Write-Error "No se encontro clang en NDK para compilar el codigo C recompilado."
        exit 1
    }

    $cSrcDir = Join-Path $RepoRoot "src-c"
    $cUnits = Get-ChildItem $cSrcDir -Filter "*-bios.*.c"
    if (-not $cUnits -or $cUnits.Count -eq 0) {
        Write-Warning "Archivos C en src-c no encontrados. Saltando compilacion AOT de C."
        return
    }

    $outDir = Join-Path $RepoRoot "target\recompiled\$Abi"
    $triple = switch ($Abi) {
        "arm64-v8a"   { "aarch64-linux-android26" }
        "armeabi-v7a" { "armv7a-linux-androideabi26" }
        "x86_64"      { "x86_64-linux-android26" }
        "x86"         { "i686-linux-android26" }
        default       { "aarch64-linux-android26" }
    }

    # Compila en paralelo cada unidad C a .o y enlaza con -Wl,-soname,libblazing_recompiled.so
    $pyCompiler = Join-Path $RepoRoot "scripts\diagnostico\compilar_c_paralelo.py"
    Write-Host "==> Compilando unidades C en paralelo para $Abi con compilar_c_paralelo.py..."
    python $pyCompiler --clang $clang --triple $triple --src-dir $cSrcDir --out-dir $outDir --abi $Abi --opt=-O1 --jobs 6
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Fallo compilacion paralela de C para $Abi"
        exit $LASTEXITCODE
    }
}

if (-not $SkipBuild) {
    Push-Location $RepoRoot
    try {
        if ($Debug) {
            if ($AllAbis) {
                Write-Host "==> Compilando .so nativo para todos los ABIs (arm64-v8a, armeabi-v7a, x86_64) con cargo ndk en modo debug..."
                cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build
            } else {
                Write-Host "==> Compilando .so nativo para $TargetAbi ($TargetArch) con cargo ndk en modo debug..."
                cargo ndk -t $TargetAbi build
            }
        } else {
            if ($AllAbis) {
                Write-Host "==> Compilando .so nativo para todos los ABIs (arm64-v8a, armeabi-v7a, x86_64) con cargo ndk en modo release..."
                cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build --release
            } else {
                Write-Host "==> Compilando .so nativo para $TargetAbi ($TargetArch) con cargo ndk en modo release..."
                cargo ndk -t $TargetAbi build --release
            }
        }
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Fallo cargo ndk build."
            exit $LASTEXITCODE
        }

        # Compilacion de C11 recompilado AOT
        Write-Host "==> Compilando codigo C11 recompilado AOT con NDK Clang..."
        if ($AllAbis) {
            Build-Recompiled-C -Abi "arm64-v8a"
            Build-Recompiled-C -Abi "armeabi-v7a"
            Build-Recompiled-C -Abi "x86_64"
        } else {
            Build-Recompiled-C -Abi $TargetAbi
        }
    } finally {
        Pop-Location
    }
}

$soFile = Join-Path $RepoRoot "target\$TargetArch\$targetProfile\libblazing_recomp.so"
if (-not (Test-Path $soFile)) {
    Write-Error "No se encontro la biblioteca compilada en $soFile."
    exit 1
}

# 7. Ensamblado base con aapt2 link
$manifestFile = Join-Path $RepoRoot "android\AndroidManifest.xml"
if (-not (Test-Path $manifestFile)) {
    Write-Error "No se encontro el manifiesto en $manifestFile."
    exit 1
}

$targetDir = Join-Path $RepoRoot "target"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$unalignedApk = Join-Path $targetDir "blazing-recomp-unaligned.apk"
$alignedApk = Join-Path $targetDir "blazing-recomp-aligned.apk"
$finalApk = Join-Path $targetDir "blazing-recomp-debug.apk"

if (Test-Path $unalignedApk) { Remove-Item $unalignedApk -Force }
if (Test-Path $alignedApk) { Remove-Item $alignedApk -Force }
if (Test-Path $finalApk) { Remove-Item $finalApk -Force }

Write-Host "==> Creando APK base con aapt2 link..."
& $aapt2 link -I $androidJar --manifest $manifestFile -o $unalignedApk
if ($LASTEXITCODE -ne 0) {
    Write-Error "aapt2 link fallo con codigo $LASTEXITCODE."
    exit $LASTEXITCODE
}

# 8. Incorporar bibliotecas nativas (.so) en lib/<abi>/ SIN COMPRESIÓN (STORED, metodo 0)
# Esto es imprescindible para carga directa por mmap (extractNativeLibs=false)
# y alineacion exacta de paginas de 16 KB requerida en Android 15.
Write-Host "==> Preparando bibliotecas nativas sin compresion..."

$stagingDir = Join-Path $targetDir "apk-staging"
if (Test-Path $stagingDir) { Remove-Item -Recurse -Force $stagingDir }
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null

# Mapeo de arquitecturas de Rust a ABIs de Android
$abiMap = @{
    "aarch64-linux-android"    = "arm64-v8a"
    "x86_64-linux-android"     = "x86_64"
    "armv7-linux-androideabi"  = "armeabi-v7a"
    "i686-linux-android"       = "x86"
}

function Stage-So-File {
    param([string]$SourcePath, [string]$DestPath)
    if (-not $NoStrip -and $llvmStrip -and (Test-Path $llvmStrip)) {
        & $llvmStrip --strip-all $SourcePath -o $DestPath
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "llvm-strip fallo, cayendo en copia directa con simbolos."
            Copy-Item $SourcePath $DestPath -Force
        }
    } else {
        Copy-Item $SourcePath $DestPath -Force
    }
}

# Siempre incluir el target principal solicitado
$abiStagingDir = Join-Path $stagingDir "lib\$TargetAbi"
New-Item -ItemType Directory -Path $abiStagingDir -Force | Out-Null
Stage-So-File -SourcePath $soFile -DestPath (Join-Path $abiStagingDir "libblazing_recomp.so")
Write-Host "   + Incluyendo $TargetAbi ($soFile)"

$recompSo = Join-Path $RepoRoot "target\recompiled\$TargetAbi\libblazing_recompiled.so"
if (Test-Path $recompSo) {
    Stage-So-File -SourcePath $recompSo -DestPath (Join-Path $abiStagingDir "libblazing_recompiled.so")
    Write-Host "   + Incluyendo AOT recomp: $TargetAbi ($recompSo)"
}

# Si existen otros ABIs ya compilados en target, incorporarlos (Fat APK multi-arquitectura)
foreach ($arch in $abiMap.Keys) {
    $abi = $abiMap[$arch]
    if ($abi -ne $TargetAbi) {
        $otherSo = Join-Path $RepoRoot "target\$arch\$targetProfile\libblazing_recomp.so"
        if (Test-Path $otherSo) {
            $otherStaging = Join-Path $stagingDir "lib\$abi"
            New-Item -ItemType Directory -Path $otherStaging -Force | Out-Null
            Stage-So-File -SourcePath $otherSo -DestPath (Join-Path $otherStaging "libblazing_recomp.so")
            Write-Host "   + Incluyendo soporte multi-ABI: $abi ($otherSo)"

            $otherRecomp = Join-Path $RepoRoot "target\recompiled\$abi\libblazing_recompiled.so"
            if (Test-Path $otherRecomp) {
                Stage-So-File -SourcePath $otherRecomp -DestPath (Join-Path $otherStaging "libblazing_recompiled.so")
                Write-Host "   + Incluyendo soporte multi-ABI AOT recomp: $abi ($otherRecomp)"
            }
        }
    }
}

try {
    if ($jar) {
        Write-Host "--> Usando jar (--no-compress) para empaquetado STORED de lib/..."
        & $jar -u -0 -M -f $unalignedApk -C $stagingDir "lib"
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Fallo jar al inyectar la biblioteca nativa."
            exit $LASTEXITCODE
        }
    } elseif ($python) {
        Write-Host "--> Usando python (zipfile.ZIP_STORED) para empaquetado STORED de lib/..."
        $pyScript = 'import zipfile, sys, os; z = zipfile.ZipFile(sys.argv[1], mode="a"); staging = sys.argv[2]; [z.write(os.path.join(r, f), arcname=os.path.relpath(os.path.join(r, f), staging).replace(chr(92), "/"), compress_type=zipfile.ZIP_STORED) for r, _, fs in os.walk(staging) for f in fs]; z.close()'
        & $python -c $pyScript $unalignedApk $stagingDir
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Fallo python al inyectar la biblioteca nativa."
            exit $LASTEXITCODE
        }
    } else {
        Write-Error "Se requiere 'jar' (JDK / Android Studio JBR) o 'python' para inyectar bibliotecas nativas sin compresion (STORED)."
        exit 1
    }
} finally {
    if (Test-Path $stagingDir) {
        Remove-Item -Recurse -Force $stagingDir -ErrorAction SilentlyContinue
    }
}

# 9. Alinear con zipalign a 4 bytes y paginas de 16 KB
Write-Host "==> Alineando APK con zipalign (16 KB page-aligned)..."
& $zipalign -f -P 16 4 $unalignedApk $alignedApk
if ($LASTEXITCODE -ne 0) {
    Write-Error "zipalign fallo con codigo $LASTEXITCODE."
    exit $LASTEXITCODE
}

& $zipalign -c -P 16 4 $alignedApk
if ($LASTEXITCODE -ne 0) {
    Write-Error "La verificacion de alineacion con zipalign fallo."
    exit $LASTEXITCODE
}

# 10. Firmar con apksigner (v1 / v2 / v3)
Write-Host "==> Firmando APK con apksigner (esquemas v1, v2 y v3)..."
& $apksigner sign --ks $debugKeystore --ks-pass pass:android --key-pass pass:android --ks-key-alias androiddebugkey --v1-signing-enabled true --v2-signing-enabled true --v3-signing-enabled true --out $finalApk $alignedApk
if ($LASTEXITCODE -ne 0) {
    Write-Error "apksigner sign fallo con codigo $LASTEXITCODE."
    exit $LASTEXITCODE
}

# 11. Verificar firma
Write-Host "==> Verificando firma del APK..."
& $apksigner verify --verbose $finalApk
if ($LASTEXITCODE -ne 0) {
    Write-Error "apksigner verify fallo con codigo $LASTEXITCODE."
    exit $LASTEXITCODE
}

# 12. Limpieza de intermedios
Remove-Item $unalignedApk, $alignedApk -ErrorAction SilentlyContinue

$apkSizeMB = [Math]::Round((Get-Item $finalApk).Length / 1MB, 2)
Write-Host "==> Exito: APK generado en $finalApk ($apkSizeMB MB)"
