# SubConv-Rust

[English](README_EN.md) | 简体中文

SubConv-Rust 是 [SubConv/SubConv](https://github.com/SubConv/SubConv) 的非官方 Rust 重写版本，用于将 Clash YAML 和受支持的分享链接转换为 Mihomo 兼容订阅。

本项目由社区独立维护，与上游项目及其维护者没有从属、官方认可或担保关系。详细来源和许可证说明见 [NOTICE.md](NOTICE.md)。

## 功能

- 转换远程订阅和单个分享链接。
- 输出基于 `proxy-providers` 的 Mihomo 配置。
- 提供 Web UI 和 HTTP API。
- 支持本地模板、远程模板和规则缓存。
- 提供预构建的 `linux/amd64` Docker 镜像。

## 使用 Docker 镜像

需要安装 Docker 和 Docker Compose。下载本项目文件后，在项目目录执行：

> **重要：启动前，运行目录中必须存在普通文件 `config.yaml`。**
> 如果该文件不存在，Docker 可能会自动创建一个名为 `config.yaml` 的目录，
> 导致容器内的 `/app/config.yaml` 不是文件，服务将启动失败。

```bash
cp config.yaml.example config.yaml
SUBCONV_IMAGE=ghcr.io/alex-syz/subconv-rust:3.0.0 \
  docker compose -f docker-compose.image.yml up -d
```

```bash
docker pull ghcr.io/alex-syz/subconv-rust:3.0.0
```

检查运行状态：

```bash
docker compose -f docker-compose.image.yml ps
curl --fail http://localhost:8080/api/v1/health
```

访问地址：

- Web UI：`http://localhost:8080`
- 健康检查：`http://localhost:8080/api/v1/health`
- 运行配置：`http://localhost:8080/config`

停止服务：

```bash
docker compose -f docker-compose.image.yml down
```

默认端口为 `8080`。需要修改宿主机端口时：

```bash
SUBCONV_PORT=3000 SUBCONV_IMAGE=ghcr.io/alex-syz/subconv-rust:3.0.0 \
  docker compose -f docker-compose.image.yml up -d
```

## 配置

最小可用配置已经包含在 [config.yaml.example](config.yaml.example) 中。通常只需复制为 `config.yaml` 即可启动。自定义模板位于 `template/` 目录。

默认模板是 `meta-rules`：

- Web UI 默认选择 `meta-rules`。
- `/sub` 请求未指定 `template` 时使用 `meta-rules`。

## API

| 路径 | 用途 |
| --- | --- |
| `GET /sub` | 转换订阅 |
| `GET /provider` | 生成代理提供者配置 |
| `GET /proxy` | 代理获取规则或模板 |
| `GET /config` | 获取运行配置 |
| `GET /api/v1/health` | 健康检查 |

## 文档

- [用户指南](docs/USER_GUIDE.md)
- [与上游的差异](docs/UPSTREAM_DIFFERENCES.md)
- [发布指南](docs/PUBLISHING.md)
- [来源及非官方声明](NOTICE.md)

## 安全提醒

- 不要在截图、日志、Issue 或文档中公开真实订阅地址。
- 不要提交 `config.yaml`、密钥或其他本地隐私配置。
- 订阅地址可能包含服务端点和访问凭据，应按敏感信息处理。

## 许可证

本项目采用 [Mozilla Public License 2.0](LICENSE) 发布。
