param(
    [string]$BumpType = "patch",
    [string]$CustomVersion = "",
    [string]$RebuildVersion = ""
)

$ErrorActionPreference = "Stop"

function Write-Utf8File {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Content,
        [switch]$NoBom
    )
    $encoding = New-Object System.Text.UTF8Encoding (-not $NoBom.IsPresent)
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function Get-CargoVersion {
    $content = Get-Content "Cargo.toml" -Raw
    if ($content -match 'version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    throw "Could not parse version from Cargo.toml"
}

function Get-ReleaseNotes {
    param(
        [string]$PrevTag,
        [string]$NewVer,
        [string]$EndRef = "HEAD"
    )

    if ($PrevTag) {
        $commitLines = git log "$PrevTag..$EndRef" --reverse --pretty=format:"- %s"
    } else {
        $commitLines = git log $EndRef --reverse --pretty=format:"- %s"
    }

    if ($commitLines) {
        return ($commitLines -join "`n")
    }
    return "- Release v$NewVer"
}

function Get-PreviousTag {
    param([string]$Tag)

    $tags = @(git tag --sort=creatordate)
    if (-not $tags) { return $null }

    $idx = [array]::IndexOf($tags, $Tag)
    if ($idx -gt 0) { return $tags[$idx - 1] }
    if ($idx -eq 0) { return $null }

    # Tag may only exist remotely; fall back to latest local tag that is not this one
    $others = @($tags | Where-Object { $_ -ne $Tag })
    if ($others.Count -gt 0) { return $others[-1] }
    return $null
}

function Get-ReleaseAssets {
    param([string]$Ver)

    $candidates = New-Object System.Collections.Generic.List[string]
    foreach ($p in @(
            "xemanh-$Ver-setup.exe",
            "installer\xemanh-$Ver-setup.exe"
        )) {
        if (Test-Path $p) { [void]$candidates.Add((Resolve-Path $p).Path) }
    }

    $debFiles = Get-ChildItem -Path "installer", "." -Filter "xemanh_${Ver}_*.deb" -ErrorAction SilentlyContinue
    foreach ($f in $debFiles) { [void]$candidates.Add($f.FullName) }

    return ,@($candidates | Select-Object -Unique)
}

function Test-GitHubReleaseExists {
    param([string]$Tag)

    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $null = & gh release view $Tag 2>&1
        return ($LASTEXITCODE -eq 0)
    } finally {
        $ErrorActionPreference = $prevEap
    }
}

function Publish-GitHubReleaseAssets {
    param(
        [string]$Ver,
        [string[]]$Assets,
        [string]$Notes = "",
        [switch]$AllowCreate
    )

    where.exe gh >$null 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "GitHub CLI (gh) not found. Please install gh or upload release files manually."
        return
    }

    $tag = "v$Ver"
    $hasAssets = $Assets -and $Assets.Count -gt 0
    $hasNotes = -not [string]::IsNullOrWhiteSpace($Notes)

    if (-not $hasAssets -and -not $hasNotes) {
        Write-Warning "No release artifacts or notes to publish."
        return
    }

    $releaseExists = Test-GitHubReleaseExists -Tag $tag

    $notesFile = $null
    if ($hasNotes) {
        $notesFile = Join-Path ([System.IO.Path]::GetTempPath()) "xemanh-v$Ver-notes.md"
        Write-Utf8File -Path $notesFile -Content $Notes
    }

    try {
        if ($releaseExists) {
            if ($hasNotes) {
                Write-Host "==> Updating release notes for $tag..." -ForegroundColor Yellow
                gh release edit $tag --notes-file $notesFile
                if ($LASTEXITCODE -ne 0) { throw "gh release edit failed for $tag" }
            }
            if ($hasAssets) {
                Write-Host "==> Updating existing GitHub Release $tag (upload --clobber)..." -ForegroundColor Yellow
                gh release upload $tag @Assets --clobber
                if ($LASTEXITCODE -ne 0) { throw "gh release upload failed for $tag" }
            }
        } elseif ($AllowCreate) {
            Write-Host "==> Creating GitHub Release via gh CLI..." -ForegroundColor Yellow
            $createArgs = @($tag)
            if ($hasAssets) { $createArgs += $Assets }
            $createArgs += @("--title", $tag)
            if ($hasNotes) {
                $createArgs += @("--notes-file", $notesFile)
            } else {
                $createArgs += @("--notes", "")
            }
            gh release create @createArgs
            if ($LASTEXITCODE -ne 0) { throw "gh release create failed for $tag" }
        } else {
            throw "GitHub release $tag does not exist. Create it first, or run a normal release."
        }
    } finally {
        if ($notesFile) {
            Remove-Item -Force $notesFile -ErrorAction SilentlyContinue
        }
    }
}

function Invoke-PackageBuild {
    param(
        [string]$Ver,
        [string]$ProjectDir
    )

    $env:APP_VERSION = $Ver

    Write-Host "==> Building Windows Binary & Installer..." -ForegroundColor Yellow
    & cmd.exe /c "build-release.bat"
    if ($LASTEXITCODE -ne 0) { throw "build-release.bat failed" }

    if (Test-Path "package.bat") {
        & cmd.exe /c "package.bat"
        if ($LASTEXITCODE -ne 0) { throw "package.bat failed" }
    }

    Write-Host "==> Building Linux .deb via WSL..." -ForegroundColor Yellow
    where.exe wsl >$null 2>&1
    if ($LASTEXITCODE -eq 0) {
        wsl --cd "$ProjectDir" bash -c "sed -i 's/\r$//' ./package-deb.sh 2>/dev/null || true; ./package-deb.sh"
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "WSL package-deb.sh failed or completed with non-zero code."
        }
    } else {
        Write-Warning "WSL is not installed/configured. Skipping .deb build."
    }
}

function Set-ProjectVersion {
    param([string]$Ver)

    $cargoContent = Get-Content "Cargo.toml" -Raw
    $cargoContent = $cargoContent -replace '(?m)^version\s*=\s*"[^"]+"', "version = `"$Ver`""
    Set-Content "Cargo.toml" -Value $cargoContent -NoNewline

    if (Test-Path "README.md") {
        $readme = Get-Content "README.md" -Raw
        $readme = $readme -replace 'xemanh-\d+\.\d+\.\d+-setup\.exe', "xemanh-$Ver-setup.exe"
        Set-Content "README.md" -Value $readme -NoNewline
    }
}

if ($PSScriptRoot) {
    $projectDir = Split-Path -Parent $PSScriptRoot
} else {
    $projectDir = (Get-Location).Path
}

$oldVer = Get-CargoVersion
Write-Host "Current version: $oldVer" -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# Rebuild existing tag: build packages + upload/clobber assets (no bump/commit)
# ---------------------------------------------------------------------------
if ($RebuildVersion -ne "" -or $BumpType.ToLower() -eq "rebuild") {
    $rebuildVer = if ($RebuildVersion -ne "") { $RebuildVersion } else { $CustomVersion }
    if ([string]::IsNullOrWhiteSpace($rebuildVer)) {
        $rebuildVer = $oldVer
        Write-Host "No version given; rebuilding current Cargo.toml version: $rebuildVer" -ForegroundColor DarkGray
    }

    $rebuildVer = $rebuildVer.Trim()
    if ($rebuildVer -match '^v(.+)$') { $rebuildVer = $Matches[1] }
    if ($rebuildVer -notmatch '^\d+\.\d+\.\d+') {
        throw "Invalid rebuild version: $rebuildVer (expected e.g. 0.1.19 or v0.1.19)"
    }

    $tag = "v$rebuildVer"
    git rev-parse --verify "refs/tags/$tag" >$null 2>&1
    if ($LASTEXITCODE -ne 0) {
        $remoteTag = git ls-remote --tags origin "refs/tags/$tag" 2>$null
        if (-not $remoteTag) {
            throw "Tag $tag not found locally or on origin. Cannot rebuild a missing tag."
        }
        Write-Warning "Tag $tag exists on origin but not locally. Continuing rebuild from current workspace."
    }

    Write-Host "Rebuild mode: $tag (no version bump, no git commit/tag)" -ForegroundColor Green

    $prevTag = Get-PreviousTag -Tag $tag
    $endRef = $tag
    git rev-parse --verify "refs/tags/$tag" >$null 2>&1
    if ($LASTEXITCODE -ne 0) { $endRef = "HEAD" }

    $releaseNotes = Get-ReleaseNotes -PrevTag $prevTag -NewVer $rebuildVer -EndRef $endRef

    Write-Host "`nRelease notes ($prevTag..$endRef):" -ForegroundColor Cyan
    Write-Host $releaseNotes -ForegroundColor DarkGray
    Write-Host ""

    if ($oldVer -ne $rebuildVer) {
        Write-Host "==> Aligning Cargo.toml/README to $rebuildVer for packaging..." -ForegroundColor Yellow
        Set-ProjectVersion -Ver $rebuildVer
    }

    Invoke-PackageBuild -Ver $rebuildVer -ProjectDir $projectDir
    $assets = Get-ReleaseAssets -Ver $rebuildVer
    Publish-GitHubReleaseAssets -Ver $rebuildVer -Assets $assets -Notes $releaseNotes -AllowCreate

    Write-Host "==> Rebuild $tag completed successfully!" -ForegroundColor Green
    return
}

# ---------------------------------------------------------------------------
# Normal release: bump → notes → build → commit/tag/push → gh release create
# ---------------------------------------------------------------------------
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
        default { throw "Invalid bump type: $BumpType (use patch, minor, major, or rebuild)" }
    }
    $newVer = "$major.$minor.$patch"
}

Write-Host "New version: $newVer" -ForegroundColor Green

$prevTag = (git tag --sort=-creatordate | Select-Object -First 1)
$releaseNotes = Get-ReleaseNotes -PrevTag $prevTag -NewVer $newVer

Write-Host "`nRelease notes ($prevTag..HEAD):" -ForegroundColor Cyan
Write-Host $releaseNotes -ForegroundColor DarkGray
Write-Host ""

Set-ProjectVersion -Ver $newVer
Invoke-PackageBuild -Ver $newVer -ProjectDir $projectDir

Write-Host "==> Creating Git Commit & Tag..." -ForegroundColor Yellow
git add Cargo.toml Cargo.lock README.md
git commit -m "chore(release): v$newVer"
git tag -a "v$newVer" -m "Release v$newVer"

Write-Host "==> Pushing to GitHub..." -ForegroundColor Yellow
git push origin master --tags

$assets = Get-ReleaseAssets -Ver $newVer
Publish-GitHubReleaseAssets -Ver $newVer -Assets $assets -Notes $releaseNotes -AllowCreate

Write-Host "==> Release $newVer completed successfully!" -ForegroundColor Green
