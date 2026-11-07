# Builds the Windows packages of Self World into target\dist\:
#   SelfWorld-Windows-Setup.exe   an installer (Start menu entry, optional desktop icon, uninstaller)
#   SelfWorld-Windows.zip         the program on its own, nothing to install
#
#   powershell -ExecutionPolicy Bypass -File scripts\package-windows.ps1
#
# Needs Rust (https://rustup.rs). Inno Setup (https://jrsoftware.org/isinfo.php)
# is installed with Chocolatey if iscc.exe is not found.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$dist = Join-Path (Get-Location) 'target\dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null

cargo build --release --bin self-world
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
$exe = Join-Path (Get-Location) 'target\release\self-world.exe'

# ---- portable zip
$stage = Join-Path (Get-Location) 'target\SelfWorld-Windows'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Path $stage | Out-Null
Copy-Item $exe (Join-Path $stage 'self-world.exe')
Copy-Item README.md $stage
Copy-Item LICENSE (Join-Path $stage 'LICENSE.txt')
Copy-Item NOTICE (Join-Path $stage 'NOTICE.txt')
Copy-Item THIRD_PARTY_LICENSES.md $stage
# small apps written in Self:  self-world.exe --app examples\todo.self
if (Test-Path examples) { Copy-Item -Recurse examples (Join-Path $stage 'examples') }
$zip = Join-Path $dist 'SelfWorld-Windows.zip'
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
Write-Host "Made $zip"

# ---- installer
$iscc = (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) {
  foreach ($p in @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe")) {
    if (Test-Path $p) { $iscc = $p; break }
  }
}
if (-not $iscc) {
  choco install innosetup -y --no-progress
  $iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
}
& $iscc "/DAppVersion=$version" "/DSourceExe=$exe" "/DOutputDir=$dist" 'assets\windows\SelfWorld.iss'
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup failed' }
Write-Host "Made $(Join-Path $dist 'SelfWorld-Windows-Setup.exe')"
