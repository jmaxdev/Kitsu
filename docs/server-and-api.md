# Persistent Local Server & REST API

Kitsu features an embedded, persistent background HTTP server daemon running locally on port `5911`. The server exposes a secure REST API (`/api/v1/*`) for IDE integrations, extensions, local web interfaces, background repository monitoring, and GitHub OAuth callback handling.

---

## 1. Daemon Architecture & Lifecycle

### Auto-Start Mechanism
When running any standard Kitsu command (e.g. `kitsu track`, `kitsu freeze`, `kitsu timeline`, `kitsu repository ...`), Kitsu performs a lightweight socket probe to `127.0.0.1:5911/api/v1/health`. If the daemon is not running, it transparently spawns a detached background process executing `kitsu server daemon --port 5911`.

### Process Management
- **Port**: `127.0.0.1:5911` (binds strictly to localhost for security).
- **Socket Engine**: Lightweight multi-threaded TCP listener using standard library sockets and `thread::spawn` connection dispatch.
- **Graceful Shutdown**: The daemon can be stopped anytime via `kitsu server off` or by sending an authorized `POST /api/v1/shutdown` request.

---

## 2. Authentication & Security Model

To prevent unauthorized local processes or browser tabs from accessing your repository data, all `/api/v1/*` endpoints (except the health probe and OAuth callback) are protected via Bearer Token authentication.

### Token Generation & Storage
1. On initial server startup, Kitsu generates a cryptographically secure 64-character hexadecimal token.
2. The token is persisted in the user's home configuration directory at `~/.kitsu/server_token` with restrictive file permissions.
3. API requests must include the header:
   ```http
   Authorization: Bearer <TOKEN>
   ```
4. Requests missing a valid Bearer token receive a `401 Unauthorized` response with `{"error": "Unauthorized"}`.

You can inspect the active token using:
```bash
kitsu server token
```

---

## 3. Global Repository Registry

The server automatically tracks all Kitsu repositories discovered across the system in `~/.kitsu/repositories.toml`.

### Tracked Metadata
For each repository, the registry records:
- **Path**: Normalized filesystem path.
- **Name**: Directory/project name.
- **Timestamps**: Registration date and last seen timestamp.
- **Remote Information**: Default remote URL and detected GitHub slug (e.g. `owner/repo`).
- **Runtime Stats**: Current HEAD checkpoint hash, active stream, total seals count, object count, storage size on disk, and issue counts.

---

## 4. REST API Endpoint Reference

### `GET /api/v1/health`
Unauthenticated health check probe.

- **Authentication**: None
- **Response**: `200 OK`
```json
{
  "status": "ok"
}
```

---

### `GET /api/v1/status`
Returns daemon runtime information, process ID, version, and port.

- **Authentication**: Required (`Bearer <token>`)
- **Response**: `200 OK`
```json
{
  "status": "running",
  "version": "0.0.4-alpha",
  "port": 5911,
  "pid": 14220
}
```

---

### `GET /api/v1/repositories`
Returns the full inventory of all globally registered repositories with runtime statistics.

- **Authentication**: Required (`Bearer <token>`)
- **Response**: `200 OK`
```json
[
  {
    "path": "E:/projects/MyProject",
    "name": "MyProject",
    "registered_at": "2026-08-30T01:20:00Z",
    "last_seen": "2026-08-30T01:35:00Z",
    "is_github": true,
    "github_repo": "username/MyProject",
    "default_remote_url": "https://github.com/username/MyProject.git",
    "details": {
      "active_persona": "Developer <dev@kitsu.dev>",
      "current_stream": "main",
      "head_hash": "634efe04905d925dfc75f073fe071865d922f0602d823015e3242142106d6c8b",
      "seals_count": 3,
      "total_objects": 48,
      "storage_bytes": 142050,
      "local_issues_count": 2
    }
  }
]
```

---

### `GET /api/v1/repositories/:id/issues`
Fetches issues for a specific repository.
- If the repository has a configured GitHub remote and authenticated credentials, returns GitHub issues.
- Otherwise, returns local issues stored in `.kitsu/issues/<id>.toml`.

- **Authentication**: Required (`Bearer <token>`)
- **Response**: `200 OK`
```json
{
  "source": "local",
  "issues": [
    {
      "id": 1,
      "title": "Fix memory allocation in CAS",
      "body": "Detailed description of the issue",
      "state": "open",
      "author": "Developer",
      "created_at": "2026-08-30T01:22:00Z",
      "closed_at": null,
      "close_comment": null,
      "comments": []
    }
  ]
}
```

---

### `GET /api/v1/repositories/:id/prs`
Fetches open pull requests for repositories connected to a GitHub remote.

- **Authentication**: Required (`Bearer <token>`)
- **Response**: `200 OK`
```json
{
  "source": "github",
  "pull_requests": [
    {
      "id": 101,
      "title": "feat: add persistent server daemon",
      "body": "Implementation of background server",
      "state": "open",
      "author": "jmaxdev",
      "head_branch": "feat-server",
      "base_branch": "main",
      "created_at": "2026-08-30T01:10:00Z",
      "url": "https://github.com/owner/repo/pull/101"
    }
  ]
}
```

---

### `GET /api/v1/github/auth?code=<OAUTH_CODE>`
OAuth callback landing endpoint. Handles browser redirect after GitHub authorization, exchanges the temporary authorization code for an OAuth access/refresh token, retrieves the user's verified noreply email (`{id}+{login}@users.noreply.github.com`), saves credentials to `~/.kitsu/github_credentials.toml`, and renders a confirmation page in the browser.

- **Authentication**: None (handled via OAuth `code` parameter)
- **Response**: `200 OK` (HTML response page)

---

### `POST /api/v1/shutdown`
Signals the background server daemon to terminate cleanly.

- **Authentication**: Required (`Bearer <token>`)
- **Response**: `200 OK`
```json
{
  "status": "shutting_down"
}
```

---

## 5. CLI Server Commands

```bash
# Check if server daemon is active
kitsu server status

# Start daemon process in background
kitsu server start

# Gracefully terminate daemon process
kitsu server off

# View current Bearer authentication token
kitsu server token
```
