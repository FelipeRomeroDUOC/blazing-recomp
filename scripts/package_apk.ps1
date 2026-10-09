# scripts/package_apk.ps1
# Arnés de empaquetado y firmado de APK para Blazing Recomp en Android.
# Cumple con la regla de portabilidad (sin rutas absolutas locales fijadas).

param (
    [string]$SdkPath = "",
    [string]$TargetAbi = "arm64-v8a",
    [string]$TargetArch = "",
    [switch]$SkipBuild = $false
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

# 1.1 Detección de Android NDK (para cargo ndk)
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

# 6. Compilar .so con cargo ndk
if (-not $SkipBuild) {
    Write-Host "==> Compilando .so nativo para $TargetAbi ($TargetArch) con cargo ndk..."
    Push-Location $RepoRoot
    try {
        cargo ndk -t $TargetAbi build
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Fallo cargo ndk build."
            exit $LASTEXITCODE
        }
    } finally {
        Pop-Location
    }
}

$soFile = Join-Path $RepoRoot "target\$TargetArch\debug\libblazing_recomp.so"
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

# 8. Incorporar libblazing_recomp.so en lib/arm64-v8a/ SIN COMPRESIÓN (STORED, metodo 0)
# Esto es imprescindible para carga directa por mmap (extractNativeLibs=false)
# y alineacion exacta de paginas de 16 KB requerida en Android 15.
Write-Host "==> Inyectando biblioteca nativa sin compresion en lib/$TargetAbi/libblazing_recomp.so..."

$stagingDir = Join-Path $targetDir "apk-staging"
if (Test-Path $stagingDir) { Remove-Item -Recurse -Force $stagingDir }
$abiStagingDir = Join-Path $stagingDir "lib\$TargetAbi"
New-Item -ItemType Directory -Path $abiStagingDir -Force | Out-Null
Copy-Item $soFile (Join-Path $abiStagingDir "libblazing_recomp.so") -Force

try {
    if ($jar) {
        Write-Host "--> Usando jar (--no-compress) para empaquetado STORED..."
        & $jar -u -0 -M -f $unalignedApk -C $stagingDir "lib/$TargetAbi/libblazing_recomp.so"
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Fallo jar al inyectar la biblioteca nativa."
            exit $LASTEXITCODE
        }
    } elseif ($python) {
        Write-Host "--> Usando python (zipfile.ZIP_STORED) para empaquetado STORED..."
        $pyScript = 'import zipfile, sys; z = zipfile.ZipFile(sys.argv[1], mode=sys.argv[4]); z.write(sys.argv[2], arcname=sys.argv[3], compress_type=zipfile.ZIP_STORED); z.close()'
        & $python -c $pyScript $unalignedApk $soFile "lib/$TargetAbi/libblazing_recomp.so" "a"
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

# 10. Firmar con apksigner (v2 / v3)
Write-Host "==> Firmando APK con apksigner..."
& $apksigner sign --ks $debugKeystore --ks-pass pass:android --key-pass pass:android --ks-key-alias androiddebugkey --v2-signing-enabled true --v3-signing-enabled true --out $finalApk $alignedApk
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
