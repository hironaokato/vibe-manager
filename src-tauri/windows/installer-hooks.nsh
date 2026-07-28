; Defense in depth for clean upgrades.
; The project build script patches Tauri's reinstall page so every version
; change must run the previous uninstaller. If an old registration still
; exists at copy time, stop instead of mixing files from two versions.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R8 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${If} $R8 != ""
    MessageBox MB_ICONSTOP|MB_OK \
      "The previous version ($R8) was not fully uninstalled. Setup will stop. Uninstall the previous version, then try again.$\r$\n$\r$\n旧バージョン ($R8) のアンインストールが完了していません。インストールを中止します。旧バージョンをアンインストールしてから、もう一度実行してください。"
    SetErrorLevel 20
    Quit
  ${EndIf}
!macroend
