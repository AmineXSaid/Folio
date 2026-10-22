; Folio's Windows installer (NSIS, Modern UI 2), built by installer/make.sh:
;   makensis -DVERSION=x.y.z installer/folio.nsi  ->  dist/Folio-Setup-x.y.z.exe
; Installs for the current user (no administrator needed) into
; %LOCALAPPDATA%\Programs\Folio, with Start menu and desktop shortcuts, an
; uninstaller and an entry in Settings > Apps. Notes live in %APPDATA%\Folio
; and stay on uninstall.

Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!define PUBLISHER "M. Amine Said"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Folio"

Name "Folio"
OutFile "..\dist\Folio-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\Folio"
InstallDirRegKey HKCU "Software\Folio" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "Folio ${VERSION} - Created and designed by ${PUBLISHER}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Folio"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "FileDescription" "Folio installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

!define MUI_ICON "..\assets\folio.ico"
!define MUI_UNICON "..\assets\folio.ico"
!define MUI_WELCOMEFINISHPAGE_BITMAP "side.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "side.bmp"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_ABORTWARNING

!define MUI_WELCOMEPAGE_TITLE "Welcome to Folio"
!define MUI_WELCOMEPAGE_TEXT "Sticky notes on endless sheets.$\r$\n$\r$\nThis installs Folio ${VERSION} for you on this computer. No administrator rights needed.$\r$\n$\r$\nClick Next to continue."
!define MUI_FINISHPAGE_TITLE "Folio is ready"
!define MUI_FINISHPAGE_TEXT "Folio is installed. You'll find it in the Start menu and on your desktop."
!define MUI_FINISHPAGE_RUN "$INSTDIR\Folio.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open Folio now"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Folio" Main
  SectionIn RO
  ; A Folio that's running would hold the file.
  nsExec::Exec 'taskkill /IM Folio.exe /F'
  SetOutPath "$INSTDIR"
  File "..\dist\Folio.exe"
  WriteUninstaller "$INSTDIR\Uninstall Folio.exe"
  CreateShortcut "$SMPROGRAMS\Folio.lnk" "$INSTDIR\Folio.exe"
  CreateShortcut "$DESKTOP\Folio.lnk" "$INSTDIR\Folio.exe"
  WriteRegStr HKCU "Software\Folio" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "Folio"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\Folio.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall Folio.exe"'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /IM Folio.exe /F'
  Delete "$INSTDIR\Folio.exe"
  Delete "$INSTDIR\Uninstall Folio.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Folio.lnk"
  Delete "$DESKTOP\Folio.lnk"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Folio"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\Folio"
SectionEnd
