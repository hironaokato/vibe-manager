; Defense in depth for clean upgrades.
; The project build script patches Tauri's reinstall page so every version
; change must run the previous uninstaller. If an old registration still
; exists at copy time, stop instead of mixing files from two versions.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R8 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${If} $R8 != ""
    MessageBox MB_ICONSTOP|MB_OK \
      "旧バージョン ($R8) のアンインストールが完了していません。$\r$\nインストールを中止します。旧バージョンをアンインストールしてから、もう一度実行してください。"
    SetErrorLevel 20
    Quit
  ${EndIf}
!macroend
