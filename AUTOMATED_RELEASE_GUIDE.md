# 🚀 Automated Binary Build & Homebrew Distribution Guide

This guide explains the complete automated workflow for building binaries and distributing via Homebrew.

---

## Overview

The project now includes **fully automated binary builds** that:
- ✅ Compile binaries for multiple platforms (macOS Intel/Apple Silicon, Linux x86_64/ARM64)
- ✅ Create GitHub Releases with all artifacts
- ✅ Automatically update Homebrew formula
- ✅ Calculate SHA256 hashes automatically
- ✅ Require zero manual intervention after tagging

---

## How It Works

```
┌─────────────────────────────────────────────┐
│  Developer creates git tag: v0.1.0          │
└──────────────┬──────────────────────────────┘
               │
               │ git push origin v0.1.0
               ▼
┌─────────────────────────────────────────────┐
│  GitHub Actions Triggered                   │
│  (.github/workflows/release.yml)            │
└──────────────┬──────────────────────────────┘
               │
               ├─────────────────┬─────────────────┐
               ▼                 ▼                 ▼
    ┌──────────────────┐ ┌──────────────┐ ┌──────────────┐
    │ macOS Intel      │ │ macOS ARM64  │ │ Linux x86_64 │
    │ Build & Package  │ │ Build & Pkg  │ │ Build & Pkg  │
    └────────┬─────────┘ └──────┬───────┘ └──────┬───────┘
             │                  │                 │
             └──────────────────┼─────────────────┘
                                ▼
                ┌───────────────────────────────┐
                │ Upload Artifacts to Release   │
                │ - env-manager-v0.1.0-*.tar.gz │
                └──────────────┬────────────────┘
                               │
                               ▼
                ┌───────────────────────────────┐
                │ Update Homebrew Formula       │
                │ - Download binary             │
                │ - Calculate SHA256            │
                │ - Update formula file         │
                │ - Commit to tap repo          │
                └──────────────┬────────────────┘
                               │
                               ▼
                ┌───────────────────────────────┐
                │ ✅ Users can install via:     │
                │ brew install env-manager      │
                └───────────────────────────────┘
```

---

## Setup Requirements

### 1. Create Homebrew Tap Repository

Create a new public repository on GitHub:
- **Name**: `homebrew-env-manager`
- **Owner**: `decentralize-mind` (or your organization)
- **URL**: `https://github.com/decentralize-mind/homebrew-env-manager`

Initialize it with:
```bash
git clone https://github.com/decentralize-mind/homebrew-env-manager.git
cd homebrew-env-manager
mkdir -p Formula
touch Formula/.gitkeep
git add .
git commit -m "Initial setup"
git push
```

### 2. Generate Personal Access Token (PAT)

The workflow needs permission to push to the tap repository:

1. Go to [GitHub Settings → Developer settings → Personal access tokens](https://github.com/settings/tokens)
2. Click **"Generate new token (classic)"**
3. Select scopes:
   - ✅ `repo` (Full control of private repositories)
4. Generate and copy the token
5. Add it as a secret to your main repository:
   - Go to: `https://github.com/decentralize-mind/env-manager/settings/secrets/actions`
   - Add new secret: **`PAT_TOKEN`** = `<your_token>`

### 3. Verify Workflow File

The workflow is already configured at `.github/workflows/release.yml`. Key features:

**Build Matrix:**
- macOS Intel (x86_64-apple-darwin)
- macOS Apple Silicon (aarch64-apple-darwin)
- Linux x86_64 (x86_64-unknown-linux-gnu)
- Linux ARM64 (aarch64-unknown-linux-gnu)

**Binary Configuration:**
- Binary name: `env-manager` (configured in `Cargo.toml`)
- Package format: `.tar.gz` archives
- Automatic caching for faster builds

---

## Release Process

### Step 1: Prepare Your Code

Make sure everything is committed:
```bash
git add .
git commit -m "Prepare for v0.1.0 release"
git push
```

### Step 2: Create and Push Tag

```bash
# Tag the release
git tag v0.1.0

# Push tag to trigger CI/CD
git push origin v0.1.0
```

### Step 3: Monitor the Build

Go to: `https://github.com/decentralize-mind/env-manager/actions`

You'll see the "Release" workflow running. It will:
1. Build binaries for all 4 platforms (~5-10 minutes)
2. Create a GitHub Release with all artifacts
3. Update the Homebrew formula automatically

### Step 4: Verify the Release

Check:
1. **GitHub Release**: `https://github.com/decentralize-mind/env-manager/releases/tag/v0.1.0`
   - Should have 4 `.tar.gz` files attached
   
2. **Homebrew Formula**: `https://github.com/decentralize-mind/homebrew-env-manager/blob/main/Formula/env-manager.rb`
   - Should be updated with new version and SHA256

### Step 5: Test Installation

```bash
# Add the tap (first time only)
brew tap decentralize-mind/env-manager

# Install
brew install env-manager

# Verify
env-manager --help
env-manager vault-init
```

---

## Updating to a New Version

For subsequent releases (v0.2.0, v0.3.0, etc.):

```bash
# 1. Make your changes
git add .
git commit -m "Add new features"
git push

# 2. Tag new version
git tag v0.2.0
git push origin v0.2.0

# 3. That's it! Everything else is automatic
```

Users can then update with:
```bash
brew update
brew upgrade env-manager
```

---

## Manual Testing (Before First Release)

Test the build process locally:

### Build for Current Platform
```bash
./build-release.sh
```

### Test the Binary
```bash
./target/release/env-manager --help
./target/release/env-manager generate
./target/release/env-manager vault-init
```

### Create Test Package
```bash
cd target/release
tar czf env-manager-test.tar.gz env-manager
shasum -a 256 env-manager-test.tar.gz
```

---

## Troubleshooting

### Problem: Workflow doesn't trigger

**Solution:**
- Ensure tag starts with `v` (e.g., `v0.1.0`, not `0.1.0`)
- Check workflow permissions in repository settings
- Verify tag was pushed: `git push origin v0.1.0`

### Problem: Build fails for specific platform

**Solution:**
- Check the workflow logs in GitHub Actions
- Common issues:
  - Missing Rust target: Add `rustup target add <target>`
  - Cross-compilation errors: Ensure proper linker is installed
  - Out of memory: Use GitHub-hosted runners (already configured)

### Problem: Homebrew formula not updated

**Solution:**
- Verify `PAT_TOKEN` secret is set correctly
- Check that tap repository exists and is accessible
- Review workflow logs for the `update-homebrew` job
- Manually verify the token has `repo` scope

### Problem: SHA256 mismatch during install

**Solution:**
- This shouldn't happen with automation, but if it does:
```bash
# Recalculate hash
curl -L -o test.tar.gz https://github.com/decentralize-mind/env-manager/releases/download/v0.1.0/env-manager-v0.1.0-aarch64-apple-darwin.tar.gz
shasum -a 256 test.tar.gz

# Compare with formula
cat $(brew --repository)/Library/Taps/decentralize-mind/homebrew-env-manager/Formula/env-manager.rb
```

### Problem: Binary name mismatch

**Solution:**
- Ensure `Cargo.toml` has the correct binary configuration:
```toml
[[bin]]
name = "env-manager"
path = "src/main.rs"
```
- Rebuild: `cargo build --release`
- Verify: `ls target/release/env-manager`

---

## Advanced Configuration

### Adding More Platforms

Edit `.github/workflows/release.yml` and add to the matrix:

```yaml
strategy:
  matrix:
    include:
      # Existing platforms...
      - os: windows-latest
        target: x86_64-pc-windows-msvc
        artifact_name: env-manager-x86_64-pc-windows-msvc.zip
```

### Customizing Build Options

Add environment variables or build flags:

```yaml
- name: Build release
  run: cargo build --release --target ${{ matrix.target }} --features "web3,vault"
  env:
    RUSTFLAGS: "-C opt-level=3"
```

### Pre-release Versions

For beta/RC releases, use tags like `v0.1.0-beta.1`:

The workflow automatically handles this. Just update the formula URL pattern if needed.

---

## Security Considerations

### 1. Binary Signing (macOS)

For production releases, consider code signing:

```yaml
- name: Sign binary (macOS)
  if: startsWith(matrix.os, 'macos')
  run: |
    codesign --sign "${{ secrets.MACOS_CERTIFICATE }}" \
             --options runtime \
             target/${{ matrix.target }}/release/env-manager
```

### 2. Checksums Verification

Publish checksums file:

```yaml
- name: Generate checksums
  run: |
    sha256sum env-manager-*.tar.gz > SHA256SUMS.txt
    
- name: Upload checksums
  uses: softprops/action-gh-release@v1
  with:
    files: SHA256SUMS.txt
```

### 3. Reproducible Builds

For maximum security, enable reproducible builds so users can verify binaries:

```yaml
- name: Build release
  run: cargo build --release --target ${{ matrix.target }}
  env:
    SOURCE_DATE_EPOCH: $(git log -1 --format=%ct)
    CARGO_INCREMENTAL: 0
```

---

## Monitoring & Maintenance

### Check Build Status

```bash
# View recent workflow runs
gh run list --workflow=release.yml

# View specific run
gh run view <run-id>
```

### Update Dependencies

Regularly update the workflow dependencies:
```bash
# Check for outdated actions
gh extension install actions/gh-actions-cache

# Update action versions in release.yml
# Change actions/checkout@v3 → actions/checkout@v4, etc.
```

### Clean Up Old Artifacts

GitHub has storage limits. Set up retention:

```yaml
- name: Upload artifact
  uses: actions/upload-artifact@v4
  with:
    name: ${{ matrix.artifact_name }}
    path: ${{ matrix.artifact_name }}
    retention-days: 30  # Delete after 30 days
```

---

## Complete Example: First Release

Here's the complete sequence for your first release:

```bash
# 1. Ensure code is ready
git status
git add .
git commit -m "Final preparation for v0.1.0"
git push

# 2. Create tap repository on GitHub (one-time)
# Go to github.com/new and create: decentralize-mind/homebrew-env-manager

# 3. Initialize tap repository
git clone https://github.com/decentralize-mind/homebrew-env-manager.git
cd homebrew-env-manager
mkdir -p Formula
echo "# Placeholder" > Formula/.gitkeep
git add .
git commit -m "Initial setup"
git push
cd ..

# 4. Create PAT token
# Go to github.com/settings/tokens and create token with 'repo' scope
# Add as PAT_TOKEN secret to env-manager repository

# 5. Tag and push
git tag v0.1.0
git push origin v0.1.0

# 6. Wait for CI/CD (5-10 minutes)
# Monitor at: github.com/decentralize-mind/env-manager/actions

# 7. Test installation
brew tap decentralize-mind/env-manager
brew install env-manager
env-manager --help

# 8. Celebrate! 🎉
```

---

## Benefits of This Setup

| Feature | Before | After |
|---------|--------|-------|
| Build Time | Manual, 30+ min | Automatic, 5-10 min |
| Platforms | 1 at a time | 4 simultaneously |
| Homebrew Update | Manual editing | Fully automatic |
| SHA256 Calculation | Manual | Automatic |
| Error Risk | High | Minimal |
| User Experience | `cargo install` (slow) | `brew install` (fast) |
| Consistency | Varies | Guaranteed |

---

## Next Steps

1. ✅ **Set up tap repository** (`homebrew-env-manager`)
2. ✅ **Create PAT token** and add as secret
3. ✅ **Tag first release** (`git tag v0.1.0 && git push origin v0.1.0`)
4. ✅ **Monitor workflow** in GitHub Actions
5. ✅ **Test installation** with Homebrew
6. ✅ **Update README** with installation instructions

Your users will then enjoy fast, reliable installations:
```bash
brew install env-manager
```

Instead of slow source builds:
```bash
cargo install --git https://github.com/decentralize-mind/env-manager
```

---

**Questions?** Check the workflow file at `.github/workflows/release.yml` for implementation details.
