# Airsend Windows Build

Preferred Windows packaging path is GitHub Actions on `windows-latest`.
The local macOS machine can verify source, frontend output, and Rust tests, but it should not be treated as a reliable producer for Windows installers because Tauri Windows packaging depends on the Windows toolchain, WebView2/MSI/NSIS behavior, and Windows runner environment.

## GitHub Actions

Workflow file:

```text
.github/workflows/windows-build.yml
```

After a GitHub remote exists, push this branch and run the workflow:

```powershell
git push -u origin codex/airsend-mvp
gh workflow run windows-build.yml --ref codex/airsend-mvp
gh run list --workflow windows-build.yml --limit 1
```

Download the artifact from the GitHub Actions run page, or with GitHub CLI after replacing `<run-id>`:

```powershell
gh run download <run-id> --name airsend-windows-<commit-sha> --dir artifacts/windows
```

Expected uploaded artifact name:

```text
airsend-windows-<commit-sha>
```

Expected files inside the artifact:

```text
src-tauri/target/release/bundle/msi/*.msi
src-tauri/target/release/bundle/nsis/*.exe
src-tauri/target/release/*.exe
```

## Windows Machine Build

Use a real Windows 11 machine or VM with Git, Node.js 22, Rust stable, and WebView2 Runtime installed.

```powershell
git clone <repo-url> airsend
cd airsend
npm ci
npm run lint
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build
```

Expected local output paths:

```text
src-tauri\target\release\bundle\msi\*.msi
src-tauri\target\release\bundle\nsis\*.exe
src-tauri\target\release\airsend.exe
```
