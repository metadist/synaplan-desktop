<!--
  Synaplan Desktop PR. Keep one PR to one concern. English only. Conventional
  Commit title (feat:/fix:/refactor:/docs:/chore:/test:/ci:). No AI attribution.
-->

## Summary

<!-- What and why (1–3 bullets). -->

## Plan reference

<!-- The DC* step(s) from the plan of record in the synaplan repo:
     _devextras/planning/20260829-desktop-agent-client/ -->

## Checklist

- [ ] `make ci-local` is green locally.
- [ ] Any user-facing string is in **all five** locales (`en/de/es/fr/tr`) in this PR.
- [ ] No secrets, API keys, keychain dumps, or pairing codes in the diff.
- [ ] No new `#[cfg(target_os)]` path/secret branch outside `synaplan-core/src/platform/`.
- [ ] No shell is constructed (C12); `make guard-no-shell` passes.
- [ ] Vendored contract fixtures unchanged (or a `protocol: 2` note explains why).
- [ ] Any deliberate platform difference is named here with the reason it is not a C10 violation.

## Cross-platform manual matrix (required from Sprint B4 / `DC18`; fill what applies)

| Check | Windows | macOS | Linux |
| ----- | :-----: | :---: | :---: |
| Installer runs without an unpassable security warning | | | |
| Pair, chat one turn, key stored in the OS secret store | | | |
| Doctor reports Python/Node/LibreOffice correctly (and on a machine without them) | | | |
| Bundled `pptx` produces a deck that opens in the native viewer | | | |
| Out-box path shown is platform-native and openable from the file manager | | | |
| Junction/symlink escape refused (screenshot) | | | |
| Autostart on/off leaves no OS entry behind (B5 only) | | | |
| Uninstall removes the app; out-box files survive | | | |
