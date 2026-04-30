# Architecture Overview

## Purpose

Codex Dynamic Meteor is a local desktop proxy that accepts Anthropic-style requests from Codex-compatible clients and routes them to a single active upstream provider.

The application combines:

- a Tauri desktop shell
- an embedded Axum HTTP proxy
- SQLite-backed local persistence
- protocol adapters for Anthropic and OpenAI-compatible upstreams

## High-Level Flow

1. Codex sends requests to the local proxy.
2. The proxy loads the currently enabled provider from SQLite-backed configuration.
3. The request is routed to the active provider.
4. If the provider uses the OpenAI protocol, request and response payloads are converted.
5. Streaming responses are normalized into Anthropic SSE events.
6. Request metadata, latency, and token usage are stored in SQLite.

## Backend Modules

### `src-tauri/src/proxy`

Owns HTTP entrypoints and routing behavior.

- `server.rs`: proxy startup, local listener, body-size limit
- `handler.rs`: request validation, upstream dispatch, streaming and non-streaming response handling
- `router.rs`: active-provider selection

### `src-tauri/src/adapter`

Owns protocol conversion.

- `anthropic.rs`: pass-through SSE monitoring
- `openai/request.rs`: Anthropic request to OpenAI chat-completions mapping
- `openai/response.rs`: OpenAI response to Anthropic response mapping
- `openai/stream.rs`: OpenAI SSE to Anthropic SSE conversion
- `openai/error.rs`: upstream error normalization

### `src-tauri/src/config`

Owns provider and app-settings persistence.

- `provider.rs`: provider model definitions
- `store.rs`: provider persistence and API-key storage
- `app_settings.rs`: app setting persistence

### `src-tauri/src/db`

Owns request log storage, stats, and migrations.

- `migration.rs`: schema creation and upgrade steps
- `logs.rs`: log read/write operations
- `stats.rs`: aggregated statistics queries

### `src-tauri/src/services`

Owns application-level orchestration.

- `provider_service.rs`: provider create/update/delete orchestration
- `log_service.rs`: log querying and export formatting

### `src-tauri/src/commands`

Thin Tauri command wrappers around services and persistence.

## Frontend Structure

The frontend is a small React application with page-level route components and shared hooks.

### Routes

- `/dashboard`
- `/providers`
- `/logs`
- `/settings`

### Hooks

Shared hooks in `src/hooks` are responsible for common data loading and polling behavior.

- `useProxyStatus`
- `useStats`
- `useProviders`
- `useLogs`
- `useAppSettings`

## Data Ownership Rules

1. Commands should stay thin.
2. Services should own orchestration and business rules.
3. Store and DB modules should focus on persistence details.
4. Adapter modules should not own provider policy.
5. Frontend pages should prefer hooks over direct Tauri calls when shared state exists.

## Provider Model

The application always routes to exactly one active provider.

- `enabled` is normalized so one provider is active when any providers exist
- `keyword` is treated as a label/group, not a routing key
- `model_mapping` specifies the actual upstream model name

## Known Constraints

1. SQLite is still accessed through a shared connection with synchronization, which is acceptable for desktop-scale throughput but not ideal for high write concurrency.
2. `proxy/handler.rs` still carries substantial orchestration complexity and remains a candidate for further service extraction.
3. Frontend hooks are improved, but not all UI state has been consolidated yet.
