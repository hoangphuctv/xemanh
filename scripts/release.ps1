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

function Get-FallbackReleaseNotes {
    param(
        [string]$PrevTag,
        [string]$NewVer
    )

    if ($PrevTag) {
        $commitLines = git log "$PrevTag..HEAD" --reverse --pretty=format:"- %s"
    } else {
        $commitLines = git log --reverse --pretty=format:"- %s"
    }

    if ($commitLines) {
        return ($commitLines -join "`n")
    }
    return "- Release v$NewVer"
}

function Find-CursorAgent {
    $agent = Get-Command agent -ErrorAction SilentlyContinue
    if ($agent) { return $agent.Source }

    $cursor = Get-Command cursor -ErrorAction SilentlyContinue
    if ($cursor) { return $cursor.Source }

    return $null
}

function Get-AgentReleaseNotes {
    param(
        [string]$PrevTag,
        [string]$NewVer,
        [string]$ProjectDir
    )

    $agentPath = Find-CursorAgent
    if (-not $agentPath) {
        Write-Warning "Cursor agent CLI not found on PATH. Falling back to commit subjects."
        return $null
    }

    $rangeLabel = if ($PrevTag) { "$PrevTag..HEAD" } else { "full history (no previous tag)" }

    if ($PrevTag) {
        $commits = (git log "$PrevTag..HEAD" --reverse --pretty=format:"%h %s" | Out-String).Trim()
        $diffStat = (git diff "$PrevTag..HEAD" --stat -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
        $diffPatch = (git diff "$PrevTag..HEAD" -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
    } else {
        $commits = (git log --reverse --pretty=format:"%h %s" | Out-String).Trim()
        $diffStat = (git diff --stat -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
        $diffPatch = (git diff -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
    }

    if (-not $commits -and -not $diffPatch) {
        Write-Warning "No commits/diff found for agent review."
        return $null
    }

    $maxDiffChars = 120000
    if ($diffPatch.Length -gt $maxDiffChars) {
        Write-Warning "git diff is large ($($diffPatch.Length) chars). Truncating for agent prompt."
        $diffPatch = $diffPatch.Substring(0, $maxDiffChars) + "`n... [diff truncated for size] ..."
    }

    $prompt = @"
You are writing GitHub Release notes for XemAnh v$NewVer.

XemAnh is a lightweight image viewer for Windows & Linux. Readers are end users (not developers).

Range: $rangeLabel

## Commits
$commits

## git diff --stat
$diffStat

## git diff
$diffPatch

Write release notes in Vietnamese, clear and friendly:
- Focus on what users gain: new features, improvements, bug fixes.
- Skip internal chores (release bumps, lockfile-only, build plumbing) unless they change user experience.
- Do NOT invent changes that are not supported by the commits/diff.
- Use GitHub markdown. Prefer sections only when they have items:
  ## Tính năng mới
  ## Cải thiện
  ## Sửa lỗi
- Short bullets; no preamble, no closing remarks, no code fences around the whole note.
- Output ONLY the release notes markdown.
"@

    Write-Host "==> Asking Cursor agent to draft release notes from git diff..." -ForegroundColor Yellow

    $agentArgs = @(
        "-p",
        "--mode", "ask",
        "--trust",
        "--workspace", $ProjectDir,
        "--output-format", "text",
        $prompt
    )

    # `cursor agent ...` when only `cursor` is on PATH
    $exeName = [System.IO.Path]::GetFileNameWithoutExtension($agentPath).ToLowerInvariant()
    if ($exeName -eq "cursor") {
        $agentArgs = @("agent") + $agentArgs
    }

    $raw = & $agentPath @agentArgs 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($raw | Out-String).Trim()

    if ($exitCode -ne 0 -or [string]::IsNullOrWhiteSpace($text)) {
        Write-Warning "Agent failed (exit $exitCode). Falling back to commit subjects."
        if ($text) { Write-Host $text -ForegroundColor DarkYellow }
        return $null
    }

    # Strip accidental outer markdown fences
    if ($text -match '(?s)^```(?:markdown|md)?\r?\n(?<body>.*)\r?\n```\s*$') {
        $text = $Matches['body'].Trim()
    }

    return $text
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

if ($PSScriptRoot) {
    $projectDir = Split-Path -Parent $PSScriptRoot
} else {
    $projectDir = (Get-Location).Path
}

$prevTag = (git tag --sort=-creatordate | Select-Object -First 1)
$releaseNotes = Get-AgentReleaseNotes -PrevTag $prevTag -NewVer $newVer -ProjectDir $projectDir
if (-not $releaseNotes) {
    $releaseNotes = Get-FallbackReleaseNotes -PrevTag $prevTag -NewVer $newVer
}

Write-Host "`nRelease notes ($prevTag..HEAD):" -ForegroundColor Cyan
Write-Host $releaseNotes -ForegroundColor DarkGray
Write-Host ""

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
        $notesFile = Join-Path ([System.IO.Path]::GetTempPath()) "xemanh-v$newVer-notes.md"
        Set-Content -Path $notesFile -Value $releaseNotes -Encoding utf8
        try {
            gh release create "v$newVer" $assets --title "v$newVer" --notes-file $notesFile
        } finally {
            Remove-Item -Force $notesFile -ErrorAction SilentlyContinue
        }
    } else {
        Write-Warning "No release artifacts found to upload."
    }
} else {
    Write-Warning "GitHub CLI (gh) not found. Please install gh or upload release files manually."
}

Write-Host "==> Release $newVer completed successfully!" -ForegroundColor Green
