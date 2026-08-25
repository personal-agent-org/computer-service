# Personal Agent Computer Service

The `pacs` command connects a computer to a Personal Agent instance and exposes the capabilities
the owner explicitly enables. It is a background capability provider and never acts as a chat
client.

The service currently provides jailed coding workspaces, filesystem operations, shell and PTY
sessions, background processes, LSP diagnostics, Git credentials, and an optional read-only home
index. Its capability announcement is extensible for computer sensors, local discovery, and other
host functions.

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
pacs enroll --server https://pa.example.com --device DEVICE_ID \
  --workspace "$HOME/projects"
pacs run
pacs tools
```

Per-user configuration is stored at
`~/.config/personal-agent/computer-service/config.toml` with mode `0600` on Unix. If no per-user
file exists, `pacs` also loads `/etc/personal-agent/computer-service/config.toml`. A per-user file
always takes precedence. Enrollment writes only the per-user file.

## Develop

```bash
just check
```
