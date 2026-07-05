# User Guide

## Prerequisites

- A working `config.yaml` created from `config.yaml.example`.
- Access to the published container image.
- A template directory containing the YAML files you want to use.

> **Important:** Run Docker Compose from a directory containing a regular
> `config.yaml` file. If it is missing, Docker may create a directory named
> `config.yaml` for the bind mount. The service will then fail to start because
> `/app/config.yaml` must be a file.

## Deploy with Docker image

```bash
cp config.yaml.example config.yaml
SUBCONV_IMAGE=ghcr.io/alex-syz/subconv-rust:3.1.0 \
  docker compose -f docker-compose.image.yml up -d
```

Open the Web UI at `http://localhost:8080`.

## Web UI workflow

1. Open `http://localhost:8080`.
2. Paste a single subscription URL or multiple URLs separated by `|`.
3. Keep the template as `meta-rules` unless you need another template.
4. Generate the subscription URL.
5. Copy the generated URL into Mihomo or another compatible client.

Example input:

```text
https://example.com/subscription
```

## Multiple subscriptions

Use `|` to combine multiple sources in one request:

```text
https://example.com/subscription-a|https://example.com/subscription-b
```

The converter splits the sources, fetches each URL, and merges the results.

## `/sub` query parameters

| Parameter | Required | Default | Notes |
| --- | --- | --- | --- |
| `url` | Yes | None | Subscription URL or `|`-separated list of URLs. |
| `template` | No | `meta-rules` | The canonical default. Omitting it is expected behavior. |
| `interval` | No | `3600` | Refresh interval in seconds. |
| `short` | No | off | Enable short output mode. |
| `npr` | No | off | Disable provider refresh? Keep the value consistent with the UI. |
| `urlstandby` | No | empty | Standby provider URLs separated by newlines. |

If a generated link does not contain `template=meta-rules`, that is intentional canonical output. The backend treats `meta-rules` as the default template.

## Configuration

The runtime configuration comes from `config.yaml` and environment variables.

Environment overrides:

- `SUBCONV_HOST`
- `SUBCONV_PORT`
- `SUBCONV_DEFAULT_TEMPLATE`
- `SUBCONV_CACHE_TTL`
- `SUBCONV_CACHE_DIR`
- `SUBCONV_CACHE_MAX_SIZE_MB`

Useful checks:

```bash
curl --fail --silent http://localhost:8080/api/v1/health
curl --fail --silent http://localhost:8080/config
```

## Troubleshooting

### `config.yaml` is missing

Create it from `config.yaml.example` before starting Docker. If the file is absent, a bind mount may create a directory instead of a file.

### Docker mounted a directory instead of a file

Check the mount source:

```bash
ls -ld config.yaml
```

If it is a directory, remove it and recreate the file from `config.yaml.example`.

### Subscription fetch fails

Use a known-good URL first and confirm network access. A failure in the input URL can cause `/sub` to return an error.

### Template load fails

Confirm the file exists in `template/` and that the selected template name matches the filename.

### Health check fails

Check the service logs and confirm the container can read `config.yaml` and `template/`.

## Safe examples

Use `http://localhost:8080` for local testing and `https://example.com/subscription` only as a fake sample URL.
