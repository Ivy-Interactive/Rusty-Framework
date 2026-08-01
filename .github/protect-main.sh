#!/usr/bin/env bash
set -euo pipefail

# Runbook: configure branch protection on main that actually gates merges
#
# This requires repo ADMIN (the active gh token has it). After this runs,
# gh pr merge --admin will no longer be able to bypass the 'build' and
# 'frontend' status checks, even for administrators.

echo "About to configure branch protection on main:"
echo "  - Required status checks: build, frontend"
echo "  - enforce_admins: true (--admin will NOT bypass)"
echo "  - No required reviews, no restrictions"
echo ""
echo "This will block merges while checks are pending."
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

gh api \
  --method PUT \
  repos/Ivy-Interactive/Rusty-Framework/branches/main/protection \
  --field required_status_checks='{"strict":false,"checks":[{"context":"build"},{"context":"frontend"}]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews=null \
  --field restrictions=null \
  --silent

echo ""
echo "Branch protection configured successfully."
echo ""
echo "Verify with:"
echo "  gh api repos/Ivy-Interactive/Rusty-Framework/branches/main/protection --jq '{checks:.required_status_checks.checks,enforce_admins:.enforce_admins.enabled}'"
echo ""
echo "To reverse (restore no protection):"
echo "  gh api -X DELETE repos/Ivy-Interactive/Rusty-Framework/branches/main/protection"
