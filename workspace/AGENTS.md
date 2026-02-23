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

## Notes
- Keep responses concise
- Use brain-server for memory lookups
- Context limit: 200K tokens
- Session scope: per-sender
