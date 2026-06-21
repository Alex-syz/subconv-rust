# SubConv-Rust

English | [简体中文](README.md)

SubConv-Rust is an unofficial Rust rewrite of [SubConv/SubConv](https://github.com/SubConv/SubConv) for converting Clash YAML and supported share links into Mihomo-compatible subscriptions.

This project is independently maintained and is not affiliated with, endorsed by, or an official release of the upstream maintainers. See [NOTICE.md](NOTICE.md) for attribution and licensing details.

## Features

- Converts remote subscriptions and standalone share links.
- Produces Mihomo configuration based on `proxy-providers`.
- Provides a Web UI and HTTP API.
- Supports local and remote templates plus rule caching.
- Provides a prebuilt `linux/amd64` Docker image.

## Run the Docker image

Install Docker and Docker Compose, download the project files, and run these commands from the project directory:

> **Important: a regular `config.yaml` file must exist in the working directory before startup.**
> If it is missing, Docker may create a directory named `config.yaml` for the bind mount. The container will then fail because `/app/config.yaml` must be a file.

```bash
cp config.yaml.example config.yaml
SUBCONV_IMAGE=ghcr.io/alex-syz/subconv-rust:3.0.0 \
  docker compose -f docker-compose.image.yml up -d
```

```bash
docker pull ghcr.io/alex-syz/subconv-rust:3.0.0
```

Verify the service:

```bash
docker compose -f docker-compose.image.yml ps
curl --fail http://localhost:8080/api/v1/health
```

- Web UI: `http://localhost:8080`
- Health check: `http://localhost:8080/api/v1/health`
- Runtime config: `http://localhost:8080/config`

Stop the service:

```bash
docker compose -f docker-compose.image.yml down
```

To use a different host port:

```bash
SUBCONV_PORT=3000 SUBCONV_IMAGE=ghcr.io/alex-syz/subconv-rust:3.0.0 \
  docker compose -f docker-compose.image.yml up -d
```

## Configuration

[config.yaml.example](config.yaml.example) contains a usable minimal configuration. Copy it to `config.yaml` before startup. Custom templates are stored in `template/`.

The default template is `meta-rules`; the Web UI and `/sub` requests without a `template` parameter both use it.

## Documentation

- [User guide](docs/USER_GUIDE.md)
- [Upstream differences](docs/UPSTREAM_DIFFERENCES.md)
- [Attribution and non-affiliation notice](NOTICE.md)

## Security

- Do not publish real subscription URLs in screenshots, logs, issues, or documentation.
- Do not commit `config.yaml`, keys, or other local secrets.
- Treat subscription URLs as sensitive information.

## License

This project is distributed under the [Mozilla Public License 2.0](LICENSE).
