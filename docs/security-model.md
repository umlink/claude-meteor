# Security Model

## Threat Model

This project is a local desktop proxy that stores upstream provider credentials and forwards model traffic on behalf of the user.

The highest-risk assets are:

- provider API keys
- request and response payloads
- local configuration injected into Codex

## Security Posture

### Local-Only Network Surface

The proxy listens on `127.0.0.1` only.

This intentionally limits exposure to the local machine. The proxy is not designed to be internet-accessible.

### Request Size Limits

The proxy enforces a request body limit at the Axum layer to reduce abuse and accidental oversized payloads.

### Provider Credential Storage

API keys are stored via:

1. system keyring when available
2. encrypted local fallback storage when keyring is unavailable

The application should never persist plaintext secrets unless all secure storage paths fail and the fallback behavior is intentionally preserved.

## Logging Policy

Allowed to log:

- provider name
- request model
- upstream URL
- protocol
- status code
- latency
- token counts
- whether an API key is empty
- API key length

Forbidden to log:

- plaintext API keys
- authorization header values
- decrypted provider secrets
- request or response payloads containing secrets

Auth-related debug output must remain at `debug` level, not `info`.

## Browser Access Policy

The proxy is not intended to be a generic browser-facing API.

Security goal:

- arbitrary web pages should not be able to consume local credentials through the proxy

The current implementation avoids permissive CORS and should keep that default.

## Data Persistence Rules

Persist:

- provider metadata
- encrypted or referenced credentials
- request metadata
- token counts
- latency
- status codes
- error summaries

Do not persist by default:

- full request payloads
- full response payloads
- sensitive tool outputs

## Remaining Risks

1. Local malware or a hostile process running under the same user can still target the localhost listener.
2. SQLite uses a local file under the user profile, so local filesystem compromise remains high impact.
3. Exported logs are user-readable artifacts and should never contain secret-bearing payloads.

## Required Future Guardrails

1. Keep auth and payload logging rules documented and enforced in code review.
2. Add tests that confirm sensitive fields are not exported or logged.
3. Consider explicit origin validation or local request authentication if the threat model becomes stricter.
