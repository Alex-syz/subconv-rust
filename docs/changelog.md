# Changelog

## 2026-06-29 — 订阅缓存 + 构建优化

### 新增功能
- **SubCache 订阅缓存**：`/sub` 和 `/provider` 端点增加内存缓存，默认 TTL 300 秒
- **请求合并**：同一 key 的并发请求只触发一次上游 fetch，避免短时间内重复请求导致上游限流（403）
- **可配置**：`SUBCONV_SUB_CACHE_TTL` 和 `SUBCONV_SUB_CACHE_LOCK_TIMEOUT`（默认 3 秒）

### 测试
- 8 个 SubCache 单元测试（含并发测试）
- 26 个 converter 解析器集成测试（覆盖 14 种协议）
- 12 个 vitest 前端测试（runtime-config、subscription-url）

### 构建优化
- Dockerfile 改用 `oven/bun:1.3.11-alpine` 直接镜像
- BuildKit cache mount 加速 Rust 编译
- 新增 `docker-compose.example.yml`（named volume 管理缓存）

### 背景
上游订阅提供商对高频请求返回 HTTP 403 限流。
原因：客户端多个订阅配置同时触发，短时间内产生多次请求。
SubCache 将同 key 请求合并为 1 次，彻底消除限流。

### Commits
```
81d091c docs: add subscription cache design spec and implementation plan
49cf1be chore: change cache eviction log from info to debug
413ca59 test: add converter parser integration tests
a33105d test: add vitest and frontend unit tests
5aeaeec chore: adopt improved Dockerfile from new version
fb65fb8 chore: remove unused start_cleanup_task function
9c2bfcc feat: wire subscription cache into /sub and /provider handlers
b50c1a9 feat: add SubCache with per-key request coalescing
13c9e72 feat: add sub_cache_ttl and sub_cache_lock_timeout config fields
```
