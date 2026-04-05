#!/bin/bash
# setup-homebrew.sh - Quick setup for Homebrew distribution

set -e

echo "🍺 Setting up Homebrew distribution for env-manager..."
echo ""

# Check if GitHub username is provided
if [ -z "$1" ]; then
    echo "❌ Usage: ./setup-homebrew.sh YOUR_GITHUB_USERNAME"
    echo ""
    echo "Example: ./setup-homebrew.sh johnsmith"
    exit 1
fi

GITHUB_USERNAME=$1
REPO_NAME="env-manager"
TAP_REPO="homebrew-${REPO_NAME}"

echo "📝 Configuration:"
echo "   GitHub Username: ${GITHUB_USERNAME}"
echo "   Main Repository: ${GITHUB_USERNAME}/${REPO_NAME}"
echo "   Tap Repository: ${GITHUB_USERNAME}/${TAP_REPO}"
echo ""

# Step 1: Update formula with correct username
echo "🔧 Updating formula files..."
sed -i '' "s/YOUR_USERNAME/${GITHUB_USERNAME}/g" env-manager.rb
sed -i '' "s/YOUR_USERNAME/${GITHUB_USERNAME}/g" .github/workflows/release.yml

echo "✅ Formula files updated"
echo ""

# Step 2: Instructions for creating tap repository
echo "📋 Next Steps:"
echo ""
echo "1️⃣  Create the tap repository on GitHub:"
echo "   - Go to: https://github.com/new"
echo "   - Repository name: ${TAP_REPO}"
echo "   - Description: Homebrew tap for env-manager"
echo "   - Make it public"
echo "   - Don't initialize with README"
echo ""

echo "2️⃣  After creating the repository, run these commands:"
echo ""
echo "   # Clone the tap repository"
echo "   git clone https://github.com/${GITHUB_USERNAME}/${TAP_REPO}.git"
echo "   cd ${TAP_REPO}"
echo ""
echo "   # Create Formula directory"
echo "   mkdir -p Formula"
echo ""
echo "   # Copy the formula file"
echo "   cp ../${REPO_NAME}/env-manager.rb Formula/env-manager.rb"
echo ""
echo "   # Commit and push"
echo "   git add Formula/env-manager.rb"
echo "   git commit -m 'Add env-manager formula'"
echo "   git push"
echo ""

echo "3️⃣  Create a Personal Access Token (PAT) for GitHub Actions:"
echo "   - Go to: https://github.com/settings/tokens"
echo "   - Click 'Generate new token (classic)'"
echo "   - Scopes: repo (full control)"
echo "   - Copy the token"
echo ""
echo "   - Go to your main repository: https://github.com/${GITHUB_USERNAME}/${REPO_NAME}/settings/secrets/actions"
echo "   - Add new secret: PAT_TOKEN = <your_token>"
echo ""

echo "4️⃣  Tag and push a release:"
echo ""
echo "   git tag v0.1.0"
echo "   git push origin v0.1.0"
echo ""
echo "   This will trigger the GitHub Actions workflow to:"
echo "   - Build binaries for all platforms"
echo "   - Create a GitHub release"
echo "   - Update the Homebrew formula automatically"
echo ""

echo "5️⃣  Test installation:"
echo ""
echo "   brew tap ${GITHUB_USERNAME}/${REPO_NAME}"
echo "   brew install env-manager"
echo "   env-manager --help"
echo ""

echo "📚 Documentation:"
echo "   - Full guide: HOMEBREW_DISTRIBUTION_GUIDE.md"
echo "   - Formula file: env-manager.rb"
echo "   - CI/CD workflow: .github/workflows/release.yml"
echo ""

echo "✨ Setup complete! Follow the steps above to publish."
echo ""
