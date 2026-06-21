# Publishing Guide

## Before you push

Review exactly what is going to be published:

```bash
git status --short
git ls-files
git diff --cached
```

Make sure `config.yaml`, local secrets, build output, and cache directories are not included.

## Create an empty GitHub repository

Set the target owner and repository name first:

```bash
export GITHUB_OWNER=your-github-name
export GITHUB_REPOSITORY=subconv-rust
```

Recommended path:

```bash
gh auth login
gh repo create "${GITHUB_OWNER}/${GITHUB_REPOSITORY}" --public --confirm
git remote add origin "https://github.com/${GITHUB_OWNER}/${GITHUB_REPOSITORY}.git"
git remote -v
git push -u origin main
```

If you use the GitHub website instead, create a brand-new empty repository and do not add a generated README, license, or `.gitignore`.

Alternative authentication paths:

- HTTPS with a personal access token.
- SSH with a configured SSH key.

Do not place tokens in the remote URL and do not commit credentials to the repository.

## Manual GHCR publication

GHCR uses the existing GitHub account; no Docker Hub account is required. Create
a classic GitHub personal access token with `write:packages`, then set:

```bash
export GITHUB_USERNAME=Alex-syz
export GHCR_IMAGE=ghcr.io/alex-syz/subconv-rust
export VERSION=3.0.0
```

Build and publish:

```bash
printf '%s' "$GHCR_TOKEN" | docker login ghcr.io --username "$GITHUB_USERNAME" --password-stdin
docker buildx build --platform linux/amd64 --load \
  -t "${GHCR_IMAGE}:${VERSION}" .
docker image inspect "${GHCR_IMAGE}:${VERSION}" \
  --format '{{.Os}}/{{.Architecture}}'
docker tag "${GHCR_IMAGE}:${VERSION}" "${GHCR_IMAGE}:latest"
docker push "${GHCR_IMAGE}:${VERSION}"
docker push "${GHCR_IMAGE}:latest"
unset GHCR_TOKEN
```

The inspected platform must be `linux/amd64`. The image contains `LICENSE`,
`NOTICE.md`, and an OCI source label pointing recipients to the public source
repository.

## Failure recovery

- Authentication rejected: create or refresh a GitHub token with `write:packages`.
- Package is private: change the package visibility to public on GitHub after its first publication.
- Existing remote: inspect `git remote -v` before changing anything.
- Duplicate tags: delete or move the conflicting image tag before re-pushing.
- Image build failure: inspect the local Buildx output.
- Health check failure: verify the container starts and responds to `/api/v1/health`.

## Image verification

After publishing, check the image metadata and run a local smoke test:

Before starting Compose, make sure the current directory contains a regular
`config.yaml` file. If it is missing, Docker may create a directory with that
name for the bind mount, causing the container to fail at startup.

```bash
docker buildx imagetools inspect "${GHCR_IMAGE}:${VERSION}"
docker pull "${GHCR_IMAGE}:${VERSION}"
cp config.yaml.example config.yaml
SUBCONV_IMAGE="${GHCR_IMAGE}:${VERSION}" \
  docker compose -f docker-compose.image.yml up -d
curl --fail --silent http://localhost:8080/api/v1/health
curl --fail --silent http://localhost:8080/config
```

## Notes

- Do not push secrets or subscription URLs.
- Do not create a GitHub repository with auto-generated starter files if you want the local history to remain clean.
