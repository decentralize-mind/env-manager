#!/bin/bash
# diagnose-release.sh - Diagnose release workflow issues

echo "🔍 Diagnosing Release Workflow Issues"
echo "======================================"
echo ""

REPO="decentralize-mind/env-manager"

# Check if tag exists
echo "1️⃣  Checking tag v0.1.0..."
TAG_EXISTS=$(curl -s https://api.github.com/repos/$REPO/git/ref/tags/v0.1.0 | jq -r '.object.sha' 2>/dev/null)
if [ "$TAG_EXISTS" != "null" ] && [ -n "$TAG_EXISTS" ]; then
    echo "   ✅ Tag v0.1.0 exists: $TAG_EXISTS"
else
    echo "   ❌ Tag v0.1.0 not found"
fi
echo ""

# Check latest release
echo "2️⃣  Checking latest release..."
RELEASE=$(curl -s https://api.github.com/repos/$REPO/releases/latest | jq -r '.tag_name' 2>/dev/null)
if [ "$RELEASE" != "null" ] && [ -n "$RELEASE" ]; then
    echo "   ✅ Latest release: $RELEASE"
else
    echo "   ❌ No releases found (workflow may have failed)"
fi
echo ""

# Check workflow runs
echo "3️⃣  Checking recent workflow runs..."
WORKFLOW_RUNS=$(curl -s "https://api.github.com/repos/$REPO/actions/workflows/release.yml/runs?per_page=1" | jq '.workflow_runs[0]' 2>/dev/null)
if [ -n "$WORKFLOW_RUNS" ]; then
    STATUS=$(echo $WORKFLOW_RUNS | jq -r '.status')
    CONCLUSION=$(echo $WORKFLOW_RUNS | jq -r '.conclusion')
    echo "   Status: $STATUS"
    echo "   Conclusion: $CONCLUSION"
    
    if [ "$CONCLUSION" == "failure" ]; then
        echo "   ❌ Workflow failed!"
        echo ""
        echo "   Common causes:"
        echo "   - PAT_TOKEN secret not configured"
        echo "   - Homebrew tap repository doesn't exist"
        echo "   - Build errors for specific platforms"
        echo ""
        echo "   Next steps:"
        echo "   1. Go to: https://github.com/$REPO/actions"
        echo "   2. Click on the failed run"
        echo "   3. Check the logs for specific error messages"
    else
        echo "   ✅ Workflow completed successfully"
    fi
else
    echo "   ⚠️  No workflow runs found"
fi
echo ""

# Check if homebrew tap repo exists
echo "4️⃣  Checking Homebrew tap repository..."
TAP_REPO_STATUS=$(curl -s -o /dev/null -w "%{http_code}" https://github.com/decentralize-mind/homebrew-env-manager)
if [ "$TAP_REPO_STATUS" == "200" ]; then
    echo "   ✅ Tap repository exists"
else
    echo "   ❌ Tap repository not found (404)"
    echo "   You need to create: https://github.com/decentralize-mind/homebrew-env-manager"
fi
echo ""

# Check secrets configuration (limited info available)
echo "5️⃣  Checking required secrets..."
echo "   Required secrets:"
echo "   - PAT_TOKEN (Personal Access Token with 'repo' scope)"
echo ""
echo "   To configure:"
echo "   1. Go to: https://github.com/$REPO/settings/secrets/actions"
echo "   2. Click 'New repository secret'"
echo "   3. Name: PAT_TOKEN"
echo "   4. Value: Your GitHub personal access token"
echo ""
echo "   To create a PAT:"
echo "   1. Go to: https://github.com/settings/tokens"
echo "   2. Generate new token (classic)"
echo "   3. Select scope: repo (full control)"
echo "   4. Copy the token"
echo ""

# Summary
echo "======================================"
echo "📊 Summary"
echo "======================================"
echo ""
echo "If the workflow failed, the most likely causes are:"
echo ""
echo "1. PAT_TOKEN not configured"
echo "   Solution: Add PAT_TOKEN secret to repository settings"
echo ""
echo "2. Homebrew tap repository doesn't exist"
echo "   Solution: Create decentralize-mind/homebrew-env-manager repository"
echo ""
echo "3. Build errors"
echo "   Solution: Check workflow logs for specific compilation errors"
echo ""
echo "View workflow logs at:"
echo "https://github.com/$REPO/actions"
echo ""
