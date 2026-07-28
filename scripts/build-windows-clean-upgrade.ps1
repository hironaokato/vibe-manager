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

    # Match the generated NSIS script by behavior rather than Tauri's comments.
    # Tauri may change comments and indentation between patch releases, while
    # these control-flow statements remain the safety-critical integration points.
    $lines = $source -split "`n"
    $updateModeCount = 0
    $updateModeIndex = -1
    $updateArgumentCount = 0

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]

        if ($line.Trim() -eq "Goto reinst_done") {
            $contextStart = [Math]::Max(0, $index - 5)
            $context = $lines[$contextStart..($index - 1)] -join "`n"
            if ($context -match '\$\{If\}\s+\$UpdateMode\s*=\s*1') {
                $indent = [regex]::Match($line, '^\s*').Value
                $lines[$index] = "${indent}Goto reinst_uninstall"
                $updateModeCount++
                $updateModeIndex = $index
                continue
            }
        }

        if (
            $line -match
            '\$\{IfThen\}.*\$UpdateMode\s*=\s*1.*StrCpy\s+\$R1\s+"\$R1 /UPDATE"'
        ) {
            $indent = [regex]::Match($line, '^\s*').Value
            $lines[$index] =
                "${indent}StrCpy `$R1 `"`$R1 /UPDATE`" ; preserve app data during every clean replacement"
            $updateArgumentCount++
        }
    }

    if ($updateModeCount -ne 1) {
        throw "Expected one Tauri update-mode jump, found $updateModeCount; refusing to build an unverified installer."
    }
    if ($updateArgumentCount -ne 1) {
        throw "Expected one Tauri uninstaller update argument, found $updateArgumentCount; refusing to build an unverified installer."
    }

    $patchedLines = [System.Collections.Generic.List[string]]::new()
    $versionBranchCount = 0
    $optOutCount = 0

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]

        # The relevant version decision is the one immediately after Tauri's
        # UpdateMode shortcut, not the earlier branch that prepares page text.
        if (
            $index -gt $updateModeIndex -and
            $line -match '^\s*\$\{If\}\s+\$R0\s*=\s*0(?:\s|;|$)'
        ) {
            $indent = [regex]::Match($line, '^\s*').Value
            [void]$patchedLines.Add("${indent}; A version change must always be a clean replacement.")
            [void]$patchedLines.Add("${indent}`${If} `$R0 <> 0")
            [void]$patchedLines.Add("${indent}  Goto reinst_uninstall")
            [void]$patchedLines.Add("${indent}`${EndIf}")
            [void]$patchedLines.Add("")
            $versionBranchCount++
        }

        if (
            $line -match
            '^\s*\$\{NSD_OnClick\}\s+\$R3\s+PageReinstallUpdateSelection\s*$'
        ) {
            $indent = [regex]::Match($line, '^\s*').Value
            [void]$patchedLines.Add("${indent}; Do not allow keeping files from another installed version.")
            [void]$patchedLines.Add("${indent}`${IfThen} `$R0 <> 0 `${|} EnableWindow `$R3 0 `${|}")
            $optOutCount++
        }

        [void]$patchedLines.Add($line)
    }

    if ($versionBranchCount -ne 1) {
        throw "Expected one Tauri version branch, found $versionBranchCount; refusing to build an unverified installer."
    }
    if ($optOutCount -ne 1) {
        throw "Expected one Tauri reinstall option, found $optOutCount; refusing to build an unverified installer."
    }

    $source = $patchedLines -join "`n"

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

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    $installerStream = $null
    try {
        $installerStream = [System.IO.File]::OpenRead($finalInstaller)
        $hashBytes = $sha256.ComputeHash($installerStream)
        $hash = [System.BitConverter]::ToString($hashBytes).Replace("-", "")
    }
    finally {
        if ($installerStream) {
            $installerStream.Dispose()
        }
        $sha256.Dispose()
    }

    Write-Output "Clean-upgrade installer: $finalInstaller"
    Write-Output "SHA256: $hash"
}
finally {
    Pop-Location
}
