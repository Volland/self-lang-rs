; Inno Setup script for the Self World installer (SelfWorld-Windows-Setup.exe).
; Built by scripts/package-windows.ps1, which passes the version and paths:
;   iscc /DAppVersion=0.2.0 /DSourceExe=..\..\target\release\self-world.exe /DOutputDir=..\..\target\dist SelfWorld.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\self-world.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\target\dist"
#endif

[Setup]
AppId={{6E2C3F4B-7A1D-4C55-9B1E-5E1F0C2D8A31}
AppName=Self World
AppVersion={#AppVersion}
AppVerName=Self World {#AppVersion}
AppPublisher=Self World
AppPublisherURL=https://volland.github.io/self-lang-rs/
AppSupportURL=https://github.com/Volland/self-lang-rs/issues
AppUpdatesURL=https://github.com/Volland/self-lang-rs/releases
DefaultDirName={autopf}\Self World
DefaultGroupName=Self World
DisableProgramGroupPage=yes
; installs for the current user without asking for administrator rights,
; or for everyone if the person chooses to
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=SelfWorld-Windows-Setup
SetupIconFile=..\icons\SelfWorld.ico
UninstallDisplayIcon={app}\self-world.exe
UninstallDisplayName=Self World
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "self-world.exe"; Flags: ignoreversion
Source: "..\icons\SelfWorld.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
; small apps written in Self:  self-world.exe --app examples\todo.self
Source: "..\..\examples\*"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\Self World"; Filename: "{app}\self-world.exe"; IconFilename: "{app}\SelfWorld.ico"
Name: "{group}\Self World examples"; Filename: "{app}\examples"
Name: "{group}\Uninstall Self World"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Self World"; Filename: "{app}\self-world.exe"; IconFilename: "{app}\SelfWorld.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\self-world.exe"; Description: "{cm:LaunchProgram,Self World}"; Flags: nowait postinstall skipifsilent
