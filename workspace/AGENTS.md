# AGENTS.md

## Agent Identity
- Name: Jetson
- Role: AI Assistant
- Model: zai/glm-4.7
- Version: 2026.2.21.2

## Core Capabilities
- Answer questions and provide information
- Help with tasks and problem-solving
- Memory integration via brain-server
- Context-aware responses

## Workspace
- **Jetson Workspace:** ~/.openclaw/workspace/ (runtime config, MEMORY.md)
- **Development Repo:** ~/openclaw-repo/ (git repo, projects, documentation)
- **Brain-server:** http://127.0.0.1:8765
- **Gateway:** http://127.0.0.1:18789

## Distributed Workflow
- **Jetson Dev:** ~/openclaw-repo/ (AI assistant works here)
- **MacBook Dev:** ~/Sites/jetson-openclaw-setup/ (Mark works here)
- **GitHub:** https://github.com/markfietje/jetson-openclaw-setup (backup)
- **Rule:** ALWAYS `git pull origin main` before making changes
- **Can edit:** Services, scripts, docs, config files directly on Jetson ✅

## Key Commands
- Health check: curl http://127.0.0.1:8765/ready
- Gateway: curl http://localhost:18789/health

## Brain Server v0.8.1 Improvements - COMPLETE ✅

### Agent 1: Security Fixes ✅ (2026-02-24)
- **Branch:** `fix/security-cors-env-config`, Commit: de168a7
- **Changes:** Replaced insecure `allow_origin(Any)` with configurable CORS via `CORS_ORIGINS` env var

### Agent 2: Dead Code Removal ✅ (2026-02-24)
- **Branch:** `fix/dead-code`, Commit: bc94347
- **Changes:** Zero clippy warnings, removed unused imports, fixed clone calls

### Agent 3: Performance Optimization ✅ (2026-02-24)
- **Branch:** `perf/search-optimization`, Commit: a044d04
- **Changes:** Added database indexes, optimized search pagination

### Agent 4: Configuration Module ✅ (2026-02-24)
- **Branch:** `refactor/config-module`, Commit: a220f44
- **Changes:** Extracted magic numbers to config.rs

### Agent 5: Testing ✅ (2026-02-24)
- **Branch:** `test/add-tests`, Commit: 067c895
- **Changes:** Added test infrastructure

### Agent 6: Error Handling & Logging ✅ (2026-02-24)
- **Branch:** `refactor/error-handling`, Commit: eacef73
- **Changes:** Added structured logging with tracing

### Agent 7: Documentation ✅ (2026-02-24)
- **Branch:** `docs/update-readme`
- **Changes:** Updated README to v0.8.1, added CORS_ORIGINS env var

### Agent 8: Release Preparation ✅ (2026-02-24)
- **Changes:** Ready for release v0.8.1

## Notes
- Keep responses concise
- Use brain-server for memory lookups
- Context limit: 200K tokens
- Session scope: per-sender
