# Docker / Compose troubleshooting (buildx / bake)

## Symptom

Some Docker Compose environments emit an error like:

> Docker Compose is configured to build using Bake, but buildx isn't installed

This happens when Compose tries to use BuildKit/Bake features that rely on the `docker buildx` plugin.

## Quick fixes

### 1) Force the classic builder (recommended)

Run Compose with BuildKit disabled:

```bash
DOCKER_BUILDKIT=0 docker compose up --build
```

### 2) Use the repo override file

This repo includes a small override intended to make it easy to avoid Bake-related
setup in toolchains that behave differently.

```bash
DOCKER_BUILDKIT=0 \
  docker compose -f docker-compose.yml -f docker-compose.nobake.yml up --build
```

## Verify buildx (optional)

If you want to enable the Bake/buildx path instead, install/enable Buildx:

```bash
docker buildx version
```

If this fails, install the `buildx` component for your Docker installation.

## Notes

- This repo's `docker-compose.yml` does not explicitly configure Bake.
- Therefore, the correct workaround is usually purely client-side (env var / builder mode).

