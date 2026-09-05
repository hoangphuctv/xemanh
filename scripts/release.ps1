param(
    [string]$BumpType = "patch",
    [string]$CustomVersion = ""
)

$ErrorActionPreference = "Stop"

function Get-CargoVersion {
    $content = Get-Content "Cargo.toml" -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    throw "Could not parse version from Cargo.toml"
}

$oldVer = Get-CargoVersion
Write-Host "Current version: $oldVer" -ForegroundColor Cyan

if ($CustomVersion -ne "") {
    $newVer = $CustomVersion
} else {
    $parts = $oldVer.Split('.')
    if ($parts.Count -lt 3) {
        throw "Invalid semver format: $oldVer"
    }
    [int]$major = $parts[0]
    [int]$minor = $parts[1]
    [int]$patch = $parts[2]

    switch ($BumpType.ToLower()) {
        "major" { $major++; $minor = 0; $patch = 0 }
        "minor" { $minor++; $patch = 0 }
        "patch" { $patch++ }
        default { throw "Invalid bump type: $BumpType (use patch, minor, major)" }
    }
    $newVer = "$major.$minor.$patch"
}

Write-Host "New version: $newVer" -ForegroundColor Green

# 1. Update Cargo.toml
$cargoContent = Get-Content "Cargo.toml" -Raw
$cargoContent = $cargoContent -replace '(?m)^version\s*=\s*"[^"]+"', "version = `"$newVer`""
Set-Content "Cargo.toml" -Value $cargoContent -NoNewline

# 2. Update README.md links if present
if (Test-Path "README.md") {
    $readme = Get-Content "README.md" -Raw
    $readme = $readme -replace 'xemanh-\d+\.\d+\.\d+-setup\.exe', "xemanh-$newVer-setup.exe"
    Set-Content "README.md" -Value $readme -NoNewline
}

$env:APP_VERSION = $newVer

# 3. Build Windows Release & Installer
Write-Host "==> Building Windows Binary & Installer..." -ForegroundColor Yellow
& cmd.exe /c "build-release.bat"
if ($LASTEXITCODE -ne 0) { throw "build-release.bat failed" }

if (Test-Path "package.bat") {
    & cmd.exe /c "package.bat"
    if ($LASTEXITCODE -ne 0) { throw "package.bat failed" }
}

# 4. Build Linux .deb via WSL
Write-Host "==> Building Linux .deb via WSL..." -ForegroundColor Yellow
where.exe wsl >$null 2>&1
if ($LASTEXITCODE -eq 0) {
    if ($PSScriptRoot) {
        $projectDir = Split-Path -Parent $PSScriptRoot
    } else {
        $projectDir = (Get-Location).Path
    }
    wsl --cd "$projectDir" bash -c "sed -i 's/\r$//' ./package-deb.sh 2>/dev/null || true; ./package-deb.sh"
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "WSL package-deb.sh failed or completed with non-zero code."
    }
} else {
    Write-Warning "WSL is not installed/configured. Skipping .deb build."
}

# 5. Create Git commit, tag & Push to GitHub Release via gh CLI
Write-Host "==> Creating Git Commit & Tag..." -ForegroundColor Yellow
git add Cargo.toml Cargo.lock README.md
git commit -m "chore(release): v$newVer"
git tag -a "v$newVer" -m "Release v$newVer"

Write-Host "==> Pushing to GitHub..." -ForegroundColor Yellow
git push origin master --tags

where.exe gh >$null 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "==> Creating GitHub Release via gh CLI..." -ForegroundColor Yellow
    $assets = @()
    $winSetup = "xemanh-$newVer-setup.exe"
    if (Test-Path $winSetup) { $assets += $winSetup }
    $winSetupInDir = "installer\xemanh-$newVer-setup.exe"
    if (Test-Path $winSetupInDir) { $assets += $winSetupInDir }
    
    $debFiles = Get-ChildItem -Path "installer", "." -Filter "xemanh_${newVer}_*.deb" -ErrorAction SilentlyContinue
    foreach ($f in $debFiles) { $assets += $f.FullName }

    if ($assets.Count -gt 0) {
        gh release create "v$newVer" $assets --title "v$newVer" --notes "Release v$newVer"
    } else {
        Write-Warning "No release artifacts found to upload."
    }
} else {
    Write-Warning "GitHub CLI (gh) not found. Please install gh or upload release files manually."
}

Write-Host "==> Release $newVer completed successfully!" -ForegroundColor Green
