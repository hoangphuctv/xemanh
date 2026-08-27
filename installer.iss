#define MyAppName "XemAnh"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "hoangphuctv"
#define MyAppExeName "xemanh.exe"
#define MyProgId "XemAnh.Image"

[Setup]
AppId={{B7E4A1D3-5F6C-4E8A-9D2B-1A3C5E7F9B04}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
OutputDir=installer
OutputBaseFilename=xemanh-{#MyAppVersion}-setup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\xemanh.ico
SetupIconFile=assets\xemanh.ico
ChangesAssociations=yes

[Tasks]
Name: "fileassoc"; Description: "Set as the &default viewer for common image files (.jpg, .png, .bmp, .gif, ...)"; GroupDescription: "File associations:"; Flags: checkedonce
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\xemanh.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\xemanh.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\xemanh.ico"; Tasks: desktopicon

[Registry]
; App path so Windows can find the exe by name
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueData: "{app}\{#MyAppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: "Path"; ValueData: "{app}"

; ProgID used as the default handler for image files
Root: HKA; Subkey: "Software\Classes\{#MyProgId}"; ValueType: string; ValueData: "XemAnh Image"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\{#MyProgId}"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "XemAnh Image"
Root: HKA; Subkey: "Software\Classes\{#MyProgId}\DefaultIcon"; ValueType: string; ValueData: "{app}\xemanh.ico,0"
Root: HKA; Subkey: "Software\Classes\{#MyProgId}\shell"; ValueType: string; ValueData: "open"
Root: HKA; Subkey: "Software\Classes\{#MyProgId}\shell\open"; ValueType: string; ValueData: "Open with {#MyAppName}"
Root: HKA; Subkey: "Software\Classes\{#MyProgId}\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

; Open With list
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\DefaultIcon"; ValueType: string; ValueData: "{app}\xemanh.ico,0"
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

; Default Programs / Settings > Apps > Default apps
Root: HKA; Subkey: "Software\{#MyAppName}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "XemAnh image viewer"
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\xemanh.ico,0"
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: "Software\{#MyAppName}\Capabilities"; Flags: uninsdeletevalue

; --- .jpg ---
Root: HKA; Subkey: "Software\Classes\.jpg"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.jpg"; ValueType: string; ValueName: "Content Type"; ValueData: "image/jpeg"
Root: HKA; Subkey: "Software\Classes\.jpg"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.jpg\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".jpg"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jpg"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.jpg\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .jpeg ---
Root: HKA; Subkey: "Software\Classes\.jpeg"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.jpeg"; ValueType: string; ValueName: "Content Type"; ValueData: "image/jpeg"
Root: HKA; Subkey: "Software\Classes\.jpeg"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".jpeg"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jpeg"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .jpe ---
Root: HKA; Subkey: "Software\Classes\.jpe"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.jpe"; ValueType: string; ValueName: "Content Type"; ValueData: "image/jpeg"
Root: HKA; Subkey: "Software\Classes\.jpe"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.jpe\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".jpe"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jpe"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.jpe\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .jfif ---
Root: HKA; Subkey: "Software\Classes\.jfif"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.jfif"; ValueType: string; ValueName: "Content Type"; ValueData: "image/jpeg"
Root: HKA; Subkey: "Software\Classes\.jfif"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.jfif\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".jfif"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jfif"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.jfif\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .png ---
Root: HKA; Subkey: "Software\Classes\.png"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.png"; ValueType: string; ValueName: "Content Type"; ValueData: "image/png"
Root: HKA; Subkey: "Software\Classes\.png"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.png\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".png"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".png"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.png\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .bmp ---
Root: HKA; Subkey: "Software\Classes\.bmp"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.bmp"; ValueType: string; ValueName: "Content Type"; ValueData: "image/bmp"
Root: HKA; Subkey: "Software\Classes\.bmp"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.bmp\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".bmp"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".bmp"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.bmp\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .dib ---
Root: HKA; Subkey: "Software\Classes\.dib"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.dib"; ValueType: string; ValueName: "Content Type"; ValueData: "image/bmp"
Root: HKA; Subkey: "Software\Classes\.dib"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.dib\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".dib"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".dib"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.dib\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .gif ---
Root: HKA; Subkey: "Software\Classes\.gif"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.gif"; ValueType: string; ValueName: "Content Type"; ValueData: "image/gif"
Root: HKA; Subkey: "Software\Classes\.gif"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.gif\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".gif"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".gif"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.gif\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

; --- .tga ---
Root: HKA; Subkey: "Software\Classes\.tga"; ValueType: string; ValueName: ""; ValueData: "{#MyProgId}"; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKA; Subkey: "Software\Classes\.tga"; ValueType: string; ValueName: "Content Type"; ValueData: "image/x-tga"
Root: HKA; Subkey: "Software\Classes\.tga"; ValueType: string; ValueName: "PerceivedType"; ValueData: "image"
Root: HKA; Subkey: "Software\Classes\.tga\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\SupportedTypes"; ValueType: string; ValueName: ".tga"; ValueData: ""
Root: HKA; Subkey: "Software\{#MyAppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".tga"; ValueData: "{#MyProgId}"
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.tga\OpenWithProgids"; ValueType: string; ValueName: "{#MyProgId}"; ValueData: ""; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Code]
const
  SHCNE_ASSOCCHANGED = $08000000;
  SHCNF_IDLIST = $0000;

procedure SHChangeNotify(wEventId: Longint; uFlags: UINT; dwItem1, dwItem2: Integer);
  external 'SHChangeNotify@shell32.dll stdcall';

procedure ClearExtUserChoice(const Ext: string);
var
  Base: string;
begin
  Base := 'Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.' + Ext;
  { Remove previous default so our ProgID becomes the handler. }
  RegDeleteKeyIncludingSubkeys(HKCU, Base + '\UserChoice');
  RegDeleteKeyIncludingSubkeys(HKCU, Base + '\UserChoiceNew');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Exts: array[0..8] of string;
  I: Integer;
begin
  if CurStep <> ssPostInstall then
    Exit;

  { Always refresh Explorer icons / Open With list. }
  SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);

  if not WizardIsTaskSelected('fileassoc') then
    Exit;

  Exts[0] := 'jpg';
  Exts[1] := 'jpeg';
  Exts[2] := 'jpe';
  Exts[3] := 'jfif';
  Exts[4] := 'png';
  Exts[5] := 'bmp';
  Exts[6] := 'dib';
  Exts[7] := 'gif';
  Exts[8] := 'tga';

  for I := 0 to GetArrayLength(Exts) - 1 do
    ClearExtUserChoice(Exts[I]);

  SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);
end;
