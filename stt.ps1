param(
    [Parameter(Position=0)]
    [ValidateSet("install", "uninstall", "start", "stop", "restart", "status", "run")]
    [string]$Action
)

$ErrorActionPreference = "Stop"
$ExeName = "simplestt.exe"
$ExePath = "$PSScriptRoot\target\release\$ExeName"
$StartMenu = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$LnkPath = "$StartMenu\simpleSTT.lnk"

function Build-Exe {
    $ucrt = "${env:ProgramFiles(x86)}\Windows Kits\10\Include\10.0.26100.0\ucrt"
    $um   = "${env:ProgramFiles(x86)}\Windows Kits\10\Include\10.0.26100.0\um"
    $shared = "${env:ProgramFiles(x86)}\Windows Kits\10\Include\10.0.26100.0\shared"
    $msvc = Get-ChildItem "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC" -Directory | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
    $msvcInclude = Join-Path $msvc "include"

    $env:BINDGEN_EXTRA_CLANG_ARGS = "-I`"$ucrt`" -I`"$um`" -I`"$shared`" -I`"$msvcInclude`""
    $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;$env:PATH"
    $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

    if (-not (Test-Path $ExePath)) {
        Write-Host "Building..." -ForegroundColor Yellow
        cargo build --release --manifest-path "$PSScriptRoot\Cargo.toml" 2>&1 | ForEach-Object {
            if ($_ -match "error") { Write-Host $_ -ForegroundColor Red }
            elseif ($_ -match "warning") { }
            elseif ($_ -match "Finished") { Write-Host $_ -ForegroundColor Green }
        }
        if (-not (Test-Path $ExePath)) { throw "Build failed" }
    }
}

function Get-ProcessStt { Get-Process -Name "simplestt" -ErrorAction SilentlyContinue }

switch ($Action) {
    "install" {
        Build-Exe
        $WshShell = New-Object -ComObject WScript.Shell
        $Shortcut = $WshShell.CreateShortcut($LnkPath)
        $Shortcut.TargetPath = $ExePath
        $Shortcut.WorkingDirectory = Split-Path $ExePath
        $Shortcut.Description = "simpleSTT - Speech to Text"
        $Shortcut.Save()
        Write-Host "Installed. Shortcut in Start Menu. Auto-starts on login if you copy the shortcut to Startup." -ForegroundColor Green
        Write-Host "  Startup folder: $env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup" -ForegroundColor Cyan
    }

    "uninstall" {
        Get-ProcessStt | Stop-Process -Force
        if (Test-Path $LnkPath) { Remove-Item $LnkPath -Force }
        $startupLnk = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\simpleSTT.lnk"
        if (Test-Path $startupLnk) { Remove-Item $startupLnk -Force }
        Write-Host "Uninstalled." -ForegroundColor Green
    }

    "start" {
        Build-Exe
        if (Get-ProcessStt) { Write-Host "Already running (PID $((Get-ProcessStt).Id))." -ForegroundColor Yellow; return }
        Start-Process $ExePath -WindowStyle Hidden
        Start-Sleep -Milliseconds 500
        if (Get-ProcessStt) {
            Write-Host "Started (PID $((Get-ProcessStt).Id)). Press F9 to toggle recording." -ForegroundColor Green
        } else {
            Write-Host "Failed to start. Run manually: $ExePath" -ForegroundColor Red
        }
    }

    "stop" {
        $proc = Get-ProcessStt
        if ($proc) {
            $proc | Stop-Process -Force
            Write-Host "Stopped." -ForegroundColor Green
        } else {
            Write-Host "Not running." -ForegroundColor Yellow
        }
    }

    "restart" {
        Get-ProcessStt | Stop-Process -Force
        Start-Sleep -Milliseconds 500
        Start-Process $ExePath -WindowStyle Hidden
        Start-Sleep -Milliseconds 500
        if (Get-ProcessStt) {
            Write-Host "Restarted (PID $((Get-ProcessStt).Id))." -ForegroundColor Green
        }
    }

    "run" {
        Build-Exe
        & $ExePath
    }

    "status" {
        $proc = Get-ProcessStt
        if ($proc) {
            Write-Host "Running (PID $($proc.Id), $([math]::Round($proc.WorkingSet64/1MB,1)) MB)" -ForegroundColor Green
        } else {
            Write-Host "Not running." -ForegroundColor Yellow
        }
    }

    default {
        Write-Host @"
simpleSTT commands:
  .\stt.ps1 run         Run in foreground (see logs)
  .\stt.ps1 start       Build if needed + start in background
  .\stt.ps1 stop        Stop running process
  .\stt.ps1 restart     Stop + start
  .\stt.ps1 status      Show if running
  .\stt.ps1 install     Build + create Start Menu shortcut
  .\stt.ps1 uninstall   Stop + remove shortcuts
"@ -ForegroundColor Cyan
    }
}
