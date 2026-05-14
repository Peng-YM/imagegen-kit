#!/bin/bash

set -e

VERSION=$(grep '^version =' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
TAG="v${VERSION}"

echo "Project version: ${VERSION}"
echo "Tag to create: ${TAG}"

if git rev-parse "${TAG}" >/dev/null 2>&1; then
  echo "Error: Tag ${TAG} already exists locally"
  exit 1
fi

if git ls-remote --tags origin | grep -q "refs/tags/${TAG}"; then
  echo "Error: Tag ${TAG} already exists on remote"
  exit 1
fi

echo "Creating tag ${TAG}..."
git tag -a "${TAG}" -m "Release ${TAG}"

echo "Pushing tag ${TAG} to origin..."
git push origin "${TAG}"

echo "Done! Tag ${TAG} has been created and pushed."
