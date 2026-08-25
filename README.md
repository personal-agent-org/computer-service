# Personal Agent Computer Service

`computer-service` connects a computer to a Personal Agent instance and exposes the capabilities
the owner explicitly enables. It is a background capability provider, not a chat client. The
desktop app and TUI live together in the [`pa`](https://github.com/personal-agent-org/pa) repo.

```text
Desktop / TUI     ── chat API ───────────► Personal Agent backend
Computer Service ◄── device WebSocket ──► Personal Agent backend
```

The service currently provides jailed coding workspaces, filesystem operations, shell and PTY
sessions, background processes, LSP diagnostics, Git credentials, and an optional read-only home
index. Its capability announcement is extensible for computer sensors, local discovery, and other
host functions without putting those responsibilities into a chat client.

## Credential boundary

Enrollment uses the OAuth device flow only to verify which user owns the registered computer. The
resulting user access token is immediately exchanged for a random, device-bound `pcs_…` service
token; user access and refresh tokens are never written to disk. The backend stores only its
SHA-256 hash and accepts the plaintext token solely for:

- the Computer Service device WebSocket; and
- the narrowly scoped Git credential-helper endpoint.

It is not a JWT and cannot access chats, runs, messages, settings, or other user APIs. Re-enrolling
rotates and immediately revokes the prior token.

## Use

```bash
computer-service enroll --server https://pa.example.com --device DEVICE_ID \
  --workspace "$HOME/projects"
computer-service run
computer-service tools
```

Configuration is stored at `~/.config/personal-agent-computer-service/config.toml` with mode `0600`
on Unix.

## Develop

```bash
just check
```
