#!/bin/bash
echo "Checking recent commits..."
git log --oneline -5

echo ""
echo "Checking for uncommitted changes..."
git status --short

echo ""
echo "Checking workflow file..."
if [ -f ".github/workflows/deploy-profile-service.yml" ]; then
  echo "✅ Workflow file exists"
  echo "Last modified: $(stat -c %y .github/workflows/deploy-profile-service.yml)"
else
  echo "❌ Workflow file not found"
fi
