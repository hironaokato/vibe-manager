$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$cargoDirectory = Join-Path $env:USERPROFILE ".cargo\bin"
$cargoExecutable = Join-Path $cargoDirectory "cargo.exe"

if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue) -and (Test-Path $cargoExecutable)) {
    $env:Path = "$cargoDirectory;$env:Path"
}

Push-Location $projectRoot
try {
    & npx tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri build failed with exit code $LASTEXITCODE."
    }

    $nsiDirectory = Join-Path $projectRoot "src-tauri\target\release\nsis\x64"
    $nsiPath = Join-Path $nsiDirectory "installer.nsi"
    if (-not (Test-Path $nsiPath)) {
        throw "Generated NSIS script was not found: $nsiPath"
    }

    $source = [System.IO.File]::ReadAllText($nsiPath).Replace("`r`n", "`n")

    $updateModeBefore = @'
  ; In update mode, always proceeds without uninstalling
  ${If} $UpdateMode = 1
    Goto reinst_done
  ${EndIf}
'@
    $updateModeAfter = @'
  ; Vibe Manager always removes the installed version before replacing it.
  ${If} $UpdateMode = 1
    Goto reinst_uninstall
  ${EndIf}
'@
    if (-not $source.Contains($updateModeBefore)) {
        throw "The expected Tauri update-mode block changed; refusing to build an unverified installer."
    }
    $source = $source.Replace($updateModeBefore, $updateModeAfter)

    $versionBranchBefore = @'
  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
'@
    $versionBranchAfter = @'
  ; A version change must always be a clean replacement.
  ${If} $R0 <> 0
    Goto reinst_uninstall
  ${EndIf}

  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
'@
    if (-not $source.Contains($versionBranchBefore)) {
        throw "The expected Tauri version branch changed; refusing to build an unverified installer."
    }
    $source = $source.Replace($versionBranchBefore, $versionBranchAfter)

    $updateArgumentBefore = '      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE'
    $updateArgumentAfter = '      StrCpy $R1 "$R1 /UPDATE" ; preserve app data during every clean replacement'
    if (-not $source.Contains($updateArgumentBefore)) {
        throw "The expected Tauri uninstaller invocation changed; refusing to build an unverified installer."
    }
    $source = $source.Replace($updateArgumentBefore, $updateArgumentAfter)

    $optOutBefore = @'
    !endif
    ${NSD_OnClick} $R3 PageReinstallUpdateSelection
'@
    $optOutAfter = @'
    !endif
    ; Do not allow keeping files from another installed version.
    ${IfThen} $R0 <> 0 ${|} EnableWindow $R3 0 ${|}
    ${NSD_OnClick} $R3 PageReinstallUpdateSelection
'@
    if (-not $source.Contains($optOutBefore)) {
        throw "The expected Tauri reinstall option changed; refusing to build an unverified installer."
    }
    $source = $source.Replace($optOutBefore, $optOutAfter)

    [System.IO.File]::WriteAllText(
        $nsiPath,
        $source,
        [System.Text.UTF8Encoding]::new($false)
    )

    $makensisCandidates = @(
        (Join-Path $env:LOCALAPPDATA "tauri\NSIS\makensis.exe"),
        (Join-Path $env:LOCALAPPDATA "tauri\NSIS\Bin\makensis.exe")
    )
    $makensis = $makensisCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $makensis) {
        throw "makensis.exe was not found in Tauri's local tool directory."
    }

    Push-Location $nsiDirectory
    try {
        & $makensis /V2 "installer.nsi"
        if ($LASTEXITCODE -ne 0) {
            throw "The clean-upgrade NSIS compile failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }

    $compiledInstaller = Join-Path $nsiDirectory "nsis-output.exe"
    if (-not (Test-Path $compiledInstaller)) {
        throw "The patched NSIS installer was not produced."
    }

    $package = Get-Content -Raw -Encoding UTF8 "package.json" | ConvertFrom-Json
    $bundleDirectory = Join-Path $projectRoot "src-tauri\target\release\bundle\nsis"
    $finalInstaller = Join-Path $bundleDirectory "Vibe Manager_$($package.version)_x64-setup.exe"
    Copy-Item -LiteralPath $compiledInstaller -Destination $finalInstaller -Force

    # Keep only the one supported installer so old versions and MSI packages
    # are not accidentally distributed together.
    $bundleRoot = [System.IO.Path]::GetFullPath(
        (Join-Path $projectRoot "src-tauri\target\release\bundle")
    )
    $bundlePrefix = $bundleRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    $finalInstallerPath = [System.IO.Path]::GetFullPath($finalInstaller)
    $staleInstallers = Get-ChildItem -LiteralPath $bundleRoot -Recurse -File |
        Where-Object {
            ($_.Extension -eq ".msi" -or $_.Name.EndsWith("-setup.exe")) -and
            ([System.IO.Path]::GetFullPath($_.FullName) -ne $finalInstallerPath)
        }
    foreach ($staleInstaller in $staleInstallers) {
        $stalePath = [System.IO.Path]::GetFullPath($staleInstaller.FullName)
        if (-not $stalePath.StartsWith($bundlePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove an installer outside the bundle directory: $stalePath"
        }
        Remove-Item -LiteralPath $stalePath -Force
    }

    $generated = [System.IO.File]::ReadAllText($nsiPath)
    $requiredMarkers = @(
        "A version change must always be a clean replacement.",
        "Goto reinst_uninstall",
        "preserve app data during every clean replacement",
        "Do not allow keeping files from another installed version.",
        "NSIS_HOOK_PREINSTALL"
    )
    foreach ($marker in $requiredMarkers) {
        if (-not $generated.Contains($marker)) {
            throw "Installer verification failed; missing marker: $marker"
        }
    }

    $hash = Get-FileHash -Algorithm SHA256 -LiteralPath $finalInstaller
    Write-Output "Clean-upgrade installer: $finalInstaller"
    Write-Output "SHA256: $($hash.Hash)"
}
finally {
    Pop-Location
}
