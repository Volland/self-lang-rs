#!/bin/sh
# Publishes Self World to GitHub and turns on its GitHub Pages site, where
# the WebAssembly version runs in the browser.
#
#   ./scripts/publish-github.sh            # creates github.com/<you>/self-lang-rs
#   ./scripts/publish-github.sh my-name    # or pick another repository name
#
# Needs the GitHub command line (gh), logged in: gh auth login
set -e
cd "$(dirname "$0")/.."
NAME="${1:-self-lang-rs}"
OWNER="$(gh api user --jq .login)"
echo "Publishing to https://github.com/$OWNER/$NAME"

if [ ! -d .git ]; then
  git init -b main
fi
git add -A
git commit -m "Self World: the Self language in Rust, with a Morphic desktop" \
  -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>" || true

if ! gh repo view "$OWNER/$NAME" >/dev/null 2>&1; then
  gh repo create "$OWNER/$NAME" --public \
    --description "The Self programming language in Rust: outliners, Morphic and a WebAssembly playground" \
    --homepage "https://$(echo "$OWNER" | tr 'A-Z' 'a-z').github.io/$NAME/" \
    --source . --remote origin
else
  git remote get-url origin >/dev/null 2>&1 || git remote add origin "git@github.com:$OWNER/$NAME.git"
fi
git push -u origin main

# GitHub Pages, built by .github/workflows/pages.yml
gh api -X POST "repos/$OWNER/$NAME/pages" -f build_type=workflow >/dev/null 2>&1 \
  || gh api -X PUT "repos/$OWNER/$NAME/pages" -f build_type=workflow >/dev/null 2>&1 || true
gh workflow run pages.yml --repo "$OWNER/$NAME" >/dev/null 2>&1 || true

echo
echo "Repository:  https://github.com/$OWNER/$NAME"
echo "Playground:  https://$(echo "$OWNER" | tr 'A-Z' 'a-z').github.io/$NAME/  (ready a few minutes after the first build)"
echo "Build log:   https://github.com/$OWNER/$NAME/actions"
