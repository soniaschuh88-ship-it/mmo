# bKG — Docker / Compose

## Problem

> Docker Compose is configured to build using Bake, but buildx isn't installed

This occurs when Compose tries to use BuildKit/Bake features.

## Fix 1 — Disable BuildKit (recommended)

```bash
DOCKER_BUILDKIT=0 docker compose up --build
```

## Fix 2 — Use the repo override file

```bash
DOCKER_BUILDKIT=0 docker compose -f docker-compose.yml -f docker-compose.nobake.yml up --build
```

## Verify buildx (optional)

```bash
docker buildx version
```

If this fails, install the `buildx` component for your Docker installation.

## Notes

- `docker-compose.yml` in this repo does not explicitly configure Bake
- The workaround is always client-side (env var / builder mode)
