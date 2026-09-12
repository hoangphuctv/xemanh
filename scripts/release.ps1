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

function Get-FallbackReleaseNotes {
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

function Find-CursorAgent {
    $agent = Get-Command agent -ErrorAction SilentlyContinue
    if ($agent) { return $agent.Source }

    $cursor = Get-Command cursor -ErrorAction SilentlyContinue
    if ($cursor) { return $cursor.Source }

    return $null
}

function Quote-ProcessArg {
    param([string]$Value)
    if ($null -eq $Value) { return '""' }
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + ($Value -replace '"', '\"') + '"'
}

function Resolve-CursorAgentNodeEntry {
    $homes = New-Object System.Collections.Generic.List[string]

    $agentCmd = Get-Command agent -ErrorAction SilentlyContinue
    if ($agentCmd -and $agentCmd.Source) {
        $src = $agentCmd.Source
        if ($src -like "*.ps1" -or $src -like "*.cmd" -or $src -like "*.bat") {
            [void]$homes.Add((Split-Path -Parent $src))
        }
    }

    if ($env:LOCALAPPDATA) {
        [void]$homes.Add((Join-Path $env:LOCALAPPDATA "cursor-agent"))
    }
    if ($env:HOME) {
        [void]$homes.Add((Join-Path $env:HOME ".local/share/cursor-agent"))
        [void]$homes.Add((Join-Path $env:HOME ".cursor-agent"))
    }
    if ($env:USERPROFILE) {
        [void]$homes.Add((Join-Path $env:USERPROFILE ".local/share/cursor-agent"))
    }

    foreach ($agentHome in ($homes | Select-Object -Unique)) {
        if (-not (Test-Path $agentHome)) { continue }

        $versionsDir = Join-Path $agentHome "versions"
        if (-not (Test-Path $versionsDir)) { continue }

        $versionName = $null
        foreach ($vf in @("version", "version.txt", "CURRENT", "active")) {
            $p = Join-Path $agentHome $vf
            if (Test-Path $p) {
                $versionName = (Get-Content $p -Raw -ErrorAction SilentlyContinue).Trim()
                if ($versionName) { break }
            }
        }
        if (-not $versionName) {
            $latest = Get-ChildItem $versionsDir -Directory -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if ($latest) { $versionName = $latest.Name }
        }
        if (-not $versionName) { continue }

        $indexJs = Join-Path $versionsDir (Join-Path $versionName "index.js")
        if (-not (Test-Path $indexJs)) { continue }

        $nodePath = $null
        foreach ($candidate in @(
                (Join-Path $agentHome "node.exe"),
                (Join-Path $agentHome "node"),
                (Join-Path (Join-Path $versionsDir $versionName) "node.exe"),
                (Join-Path (Join-Path $versionsDir $versionName) "node")
            )) {
            if (Test-Path $candidate) { $nodePath = $candidate; break }
        }
        if (-not $nodePath) {
            $bundled = Get-ChildItem $agentHome -Filter "node.exe" -Recurse -ErrorAction SilentlyContinue |
                Select-Object -First 1
            if ($bundled) { $nodePath = $bundled.FullName }
        }
        if (-not $nodePath) {
            $nodeCmd = Get-Command node -ErrorAction SilentlyContinue
            if ($nodeCmd) { $nodePath = $nodeCmd.Source }
        }
        if (-not $nodePath) { continue }

        return @{ Node = $nodePath; Entry = $indexJs; Home = $agentHome }
    }

    return $null
}

function Invoke-AgentUtf8 {
    param(
        [string[]]$AgentArgs,
        [string]$ProjectDir
    )

    $utf8 = New-Object System.Text.UTF8Encoding $false
    $entry = Resolve-CursorAgentNodeEntry
    if ($entry) {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $entry.Node
        $allArgs = @($entry.Entry) + $AgentArgs
        $psi.Arguments = ($allArgs | ForEach-Object { Quote-ProcessArg $_ }) -join " "
        $psi.WorkingDirectory = $ProjectDir
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.CreateNoWindow = $true
        $psi.StandardOutputEncoding = $utf8
        $psi.StandardErrorEncoding = $utf8

        $proc = New-Object System.Diagnostics.Process
        $proc.StartInfo = $psi
        [void]$proc.Start()
        $stdout = $proc.StandardOutput.ReadToEnd()
        $stderr = $proc.StandardError.ReadToEnd()
        $proc.WaitForExit()

        return @{
            ExitCode = $proc.ExitCode
            Text     = $stdout.Trim()
            StdErr   = $stderr.Trim()
        }
    }

    # Fallback: PowerShell pipeline with UTF-8 console decoding (may still mangle on some hosts)
    $agentPath = Find-CursorAgent
    if (-not $agentPath) {
        return $null
    }

    $exeName = [System.IO.Path]::GetFileNameWithoutExtension($agentPath).ToLowerInvariant()
    $invokeArgs = $AgentArgs
    if ($exeName -eq "cursor") {
        $invokeArgs = @("agent") + $AgentArgs
    }

    $prevEap = $ErrorActionPreference
    $prevOutEnc = [Console]::OutputEncoding
    $prevOutputEncoding = $OutputEncoding
    $ErrorActionPreference = "Continue"
    try {
        try { cmd /c "chcp 65001 >nul" | Out-Null } catch { }
        [Console]::OutputEncoding = $utf8
        $OutputEncoding = $utf8
        $raw = & $agentPath @invokeArgs 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $prevEap
        [Console]::OutputEncoding = $prevOutEnc
        $OutputEncoding = $prevOutputEncoding
    }

    $text = (
        $raw |
        ForEach-Object {
            if ($_ -is [System.Management.Automation.ErrorRecord]) { $_.ToString() } else { "$_" }
        } |
        Out-String
    ).Trim()

    return @{
        ExitCode = $exitCode
        Text     = $text
        StdErr   = ""
    }
}

function Get-AgentReleaseNotes {
    param(
        [string]$PrevTag,
        [string]$NewVer,
        [string]$ProjectDir,
        [string]$EndRef = "HEAD"
    )

    if (-not (Find-CursorAgent) -and -not (Resolve-CursorAgentNodeEntry)) {
        Write-Warning "Cursor agent CLI not found on PATH. Falling back to commit subjects."
        return $null
    }

    $rangeLabel = if ($PrevTag) { "$PrevTag..$EndRef" } else { "$EndRef (no previous tag)" }

    if ($PrevTag) {
        $commits = (git log "$PrevTag..$EndRef" --reverse --pretty=format:"%h %s" | Out-String).Trim()
        $diffStat = (git diff "$PrevTag..$EndRef" --stat -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
        $diffPatch = (git diff "$PrevTag..$EndRef" -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
    } else {
        $commits = (git log $EndRef --reverse --pretty=format:"%h %s" | Out-String).Trim()
        $diffStat = (git diff "$EndRef" --stat -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
        $diffPatch = (git diff "$EndRef" -- . ":(exclude)Cargo.lock" ":(exclude)target" ":(exclude).bridge-output" | Out-String).Trim()
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

    # Write the full prompt to a file. Passing a large multiline prompt (with diff
    # lines starting with "-") as CLI argv gets split and misparsed as options.
    $requestFile = Join-Path $ProjectDir ".release-notes-request.md"
    $requestBody = @"
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
    Write-Utf8File -Path $requestFile -Content $requestBody

    # Short single-line prompt only — no leading dashes, no newlines.
    $shortPrompt = "Read the file .release-notes-request.md in the workspace root and follow its instructions exactly."

    Write-Host "==> Asking Cursor agent to draft release notes from git diff..." -ForegroundColor Yellow

    $agentArgs = @(
        "-p",
        "--mode", "ask",
        "--trust",
        "--workspace", $ProjectDir,
        "--output-format", "text",
        "--",
        $shortPrompt
    )

    try {
        $result = Invoke-AgentUtf8 -AgentArgs $agentArgs -ProjectDir $ProjectDir
    } finally {
        Remove-Item -Force $requestFile -ErrorAction SilentlyContinue
    }

    if (-not $result) {
        Write-Warning "Could not invoke Cursor agent. Falling back to commit subjects."
        return $null
    }

    $text = $result.Text
    if ($result.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($text)) {
        Write-Warning "Agent failed (exit $($result.ExitCode)). Falling back to commit subjects."
        if ($result.StdErr) { Write-Host $result.StdErr -ForegroundColor DarkYellow }
        if ($text) { Write-Host $text -ForegroundColor DarkYellow }
        return $null
    }

    # Strip accidental outer markdown fences
    if ($text -match '(?s)^```(?:markdown|md)?\r?\n(?<body>.*)\r?\n```\s*$') {
        $text = $Matches['body'].Trim()
    }

    # Detect classic UTF-8-as-CP437 mojibake (e.g. T├¡nh)
    if ($text -match '[├─└┌ß╞╗]') {
        Write-Warning "Agent output looks encoding-corrupted. Falling back to commit subjects."
        return $null
    }

    return $text
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

    $releaseExists = $false
    gh release view $tag >$null 2>&1
    if ($LASTEXITCODE -eq 0) { $releaseExists = $true }

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

    $releaseNotes = Get-AgentReleaseNotes -PrevTag $prevTag -NewVer $rebuildVer -ProjectDir $projectDir -EndRef $endRef
    if (-not $releaseNotes) {
        $releaseNotes = Get-FallbackReleaseNotes -PrevTag $prevTag -NewVer $rebuildVer -EndRef $endRef
    }

    Write-Host "`nRelease notes ($prevTag..$endRef):" -ForegroundColor Cyan
    Write-Host $releaseNotes -ForegroundColor DarkGray
    Write-Host ""

    if ($oldVer -ne $rebuildVer) {
        Write-Host "==> Aligning Cargo.toml/README to $rebuildVer for packaging..." -ForegroundColor Yellow
        Set-ProjectVersion -Ver $rebuildVer
    }

    Invoke-PackageBuild -Ver $rebuildVer -ProjectDir $projectDir
    $assets = Get-ReleaseAssets -Ver $rebuildVer
    Publish-GitHubReleaseAssets -Ver $rebuildVer -Assets $assets -Notes $releaseNotes

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
$releaseNotes = Get-AgentReleaseNotes -PrevTag $prevTag -NewVer $newVer -ProjectDir $projectDir
if (-not $releaseNotes) {
    $releaseNotes = Get-FallbackReleaseNotes -PrevTag $prevTag -NewVer $newVer
}

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
