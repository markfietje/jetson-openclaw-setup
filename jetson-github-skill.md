---
name: github
description: "GitHub and Git operations for the Coding Factory. Use for: version control, repository management, code collaboration, CI/CD automation, and documentation standards."
---

# GitHub Coding Factory Skill

OpenClaw's GitHub integration for professional software development, version control, and collaborative coding.

## 🎯 Mission

Transform OpenClaw into a productive Coding Factory with:
- Professional Git workflow
- Secure GitHub operations
- Automated documentation
- Semantic versioning
- CI/CD integration

## 📁 Workspace Context

```
Development Repo: ~/openclaw-repo/          # Main coding directory
GitHub Remote: git@github.com:markfietze/jetson-openclaw-setup.git
SSH Key: ~/.ssh/id_ed25519_github           # GitHub authentication
Branch Strategy: main (protected) + feature branches
```

## 🚀 Core Git Operations

### Initial Setup (already configured ✅)
```bash
# Already configured - for reference
cd ~/openclaw-repo/
git remote origin git@github.com:markfietze/jetson-openclaw-setup.git
git config user.email "jetson@jetson"
git config user.name "Jetson OpenClaw"
```

### Daily Workflow
```bash
# 1. Always pull before starting work
cd ~/openclaw-repo/
git pull origin main

# 2. Create feature branch for new work
git checkout -b feature/your-feature-name

# 3. Make changes and commit
git add .
git commit -m "feat: descriptive commit message"

# 4. Push to GitHub
git push origin feature/your-feature-name

# 5. Pull updates from main regularly
git pull origin main
```

## 📝 Commit Message Standards

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

**Examples:**
```
feat: add webhook endpoint for signal processing
fix: resolve memory leak in brain-server
docs: update API documentation for signal-gateway
refactor: simplify error handling in signal-gateway
test: add unit tests for webhook validation
```

## 🔒 Security Practices

### Pre-commit Security Checks
```bash
# Check for sensitive data before committing
git diff --cached | grep -i "password\|secret\|api_key\|token"

# Never commit:
# - SSH keys (*.pem, *.key, id_*)
# - Database files (*.db, *.db-shm, *.db-wal)
# - Credentials files
# - Environment variables with secrets
```

### Already Configured (✅)
- SSH key authentication (no passwords)
- IP restrictions on authorized_keys
- GPG signing can be added if needed

## 📚 Documentation Standards

### Project Structure
```
README.md                    # Project overview (always keep updated)
CHANGELOG.md                 # Version history
AGENTS.md                    # Agent capabilities
docs/
├── architecture.md          # System architecture
├── api/                     # API documentation
├── deployment.md            # Deployment procedures
└── troubleshooting.md       # Common issues
```

### README Requirements
Every service must have:
```markdown
# Service Name

## Quick Start
## Installation
## Configuration
## API Reference
## Development
## Testing
## Deployment
## Troubleshooting
## Changelog
```

## 🔢 Semantic Versioning

Format: `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

**Examples:**
- `1.0.0` → Initial release
- `1.1.0` → Add new feature
- `1.1.1` → Bug fix
- `2.0.0` → Breaking changes

### Tagging Releases
```bash
# After merging to main
cd ~/openclaw-repo/
git tag -a v1.0.0 -m "Release v1.0.0: Initial stable release"
git push origin v1.0.0
```

## 🔄 Branch Strategy

### Main Branches
- `main`: Production-ready code (protected)
- `develop`: Integration branch (optional)

### Feature Branches
```bash
# Naming: feature/description
feature/signal-webhook
feature/brain-api-v2
feature/documentation-update
```

### Bugfix Branches
```bash
# Naming: fix/description
fix/memory-leak
fix/api-authentication
```

## 🤖 GitHub API Operations

### Using GitHub CLI (if installed)
```bash
# List issues
gh issue list

# Create issue
gh issue create --title "Bug in signal processing" --body "Description"

# Create PR
gh pr create --title "Feature: Webhook support" --body "Description"

# View releases
gh release list
```

### Using Git (always available)
```bash
# List remote branches
git branch -r

# Track remote branch
git checkout --track origin/feature-branch

# View commit history
git log --oneline --graph --all
```

## 🚦 CI/CD Integration

### GitHub Actions (Auto-deploy ✅)
```yaml
# Already configured in .github/workflows/
- auto-deploy.yml          # Builds ARM64 and deploys to Jetson
- brain-server-ci.yml      # Tests brain-server
```

### Deployment Workflow
1. Push code to GitHub
2. GitHub Actions builds ARM64 binary
3. Auto-deploys to Jetson via SSH
4. Services restart automatically
5. Health checks validate deployment

### Manual Deployment
```bash
# From MacBook
cd ~/Sites/jetson-openclaw-setup/
git push origin main

# From Jetson
cd ~/openclaw-repo/
git pull origin main
# GitHub Actions auto-deploys
```

## 🧪 Testing Standards

### Before Committing
```bash
# Rust services
cd ~/openclaw-repo/services/brain-server/
cargo test
cargo clippy

# Python scripts
python -m pytest tests/

# Shell scripts
shellcheck script.sh
```

### Pre-commit Checklist
- [ ] Code compiles/runs without errors
- [ ] Tests pass
- [ ] Documentation updated
- [ ] No sensitive data committed
- [ ] Commit message follows conventions
- [ ] Changes reviewed (if working with team)

## 📊 Project Status Commands

```bash
# Repository health
cd ~/openclaw-repo/
git status                    # Current status
git log --oneline -10         # Recent commits
git branch -a                 # All branches
git remote -v                 # Remotes

# Sync status
git log HEAD..origin/main     # Commits on main not in local
git log origin/main..HEAD     # Commits in local not on main
```

## 🎯 Coding Factory Best Practices

### 1. Atomic Commits
```bash
# Good: One logical change per commit
git add services/signal-gateway/src/webhook.rs
git commit -m "feat: add webhook POST endpoint"

# Avoid: Giant commits with unrelated changes
```

### 2. Frequent Sync
```bash
# Pull from main at least daily
git pull origin main

# Push feature branches frequently
git push origin feature-name
```

### 3. Code Review
```bash
# Before merging to main:
# 1. Ensure tests pass
# 2. Review changes
# 3. Update documentation
# 4. Create PR (if team collaboration)
# 5. Merge after approval
```

### 4. Documentation-First
```bash
# Update docs before coding
# Document decisions in CHANGELOG.md
# Keep README.md current
# Comment complex logic
```

## 🛠️ Troubleshooting

### Merge Conflicts
```bash
# Pull with rebase to maintain clean history
git pull origin main --rebase

# Resolve conflicts
git add .
git rebase --continue

# If needed, abort and try merge
git rebase --abort
git pull origin main  # Regular merge
```

### Undo Mistakes
```bash
# Undo last commit (keep changes)
git reset HEAD~1

# Undo last commit (discard changes)
git reset --hard HEAD~1

# Undo changes to file
git checkout -- file.txt

# Undo staged changes
git reset HEAD file.txt
```

### Sync Issues
```bash
# View divergent branches
git log --oneline --graph --all

# Force push (use carefully!)
git push origin feature-name --force-with-lease
```

## 📦 Repository Management

### Creating New Services
```bash
cd ~/openclaw-repo/
mkdir services/new-service
cd services/new-service

# Initialize Rust service
cargo init --name new-service

# Create README
echo "# New Service" > README.md

# Commit and push
git add .
git commit -m "feat: initialize new service scaffold"
git push origin main
```

### Adding Documentation
```bash
# Create docs directory
mkdir -p docs/api

# Add documentation
echo "# API Documentation" > docs/api/README.md

# Commit
git add docs/
git commit -m "docs: add API documentation structure"
git push origin main
```

## 🎓 Learning Resources

### Git Commands Reference
```bash
# Essential commands
git clone      # Clone repository
git init       # Initialize repository
git status     # Show working tree status
git add        # Stage changes
git commit     # Commit changes
git push       # Push to remote
git pull       # Pull from remote
git branch     # Manage branches
git checkout   # Switch branches
git merge      # Merge branches
git log        # Show commit history
git diff       # Show changes
git stash      # Stash changes
git tag        # Manage tags
```

### GitHub Workflow
1. **Plan**: Create issue/feature request
2. **Develop**: Create feature branch
3. **Test**: Write and run tests
4. **Document**: Update docs and README
5. **Commit**: Follow conventional commits
6. **Push**: Push to GitHub
7. **Review**: Code review (if team)
8. **Merge**: Merge to main
9. **Deploy**: CI/CD auto-deploys

## 🚀 Advanced Features

### Git Hooks (Optional)
```bash
# Pre-commit hook for code quality
# Create .git/hooks/pre-commit
#!/bin/bash
cargo test && cargo clippy
```

### Submodules (if needed)
```bash
# Add submodule
git submodule add https://github.com/user/repo.git libs/repo

# Update submodules
git submodule update --remote
```

### Git Worktrees (for parallel work)
```bash
# Create worktree
git worktree add ../feature-branch feature-branch

# List worktrees
git worktree list

# Remove worktree
git worktree remove ../feature-branch
```

## 📋 Quick Reference

### Daily Commands
```bash
cd ~/openclaw-repo/
git pull origin main              # Sync with latest
# ... make changes ...
git add .                         # Stage changes
git commit -m "type: description" # Commit
git push origin main              # Push to GitHub
```

### Feature Branch Workflow
```bash
git checkout -b feature/new-feature  # Create branch
# ... develop feature ...
git add .
git commit -m "feat: add new feature"
git push origin feature/new-feature  # Push branch
```

### Emergency Fixes
```bash
git checkout main                   # Switch to main
git pull origin main                # Sync
git checkout -b hotfix/urgent-fix   # Create hotfix
# ... fix issue ...
git add .
git commit -m "fix: urgent issue"
git push origin hotfix/urgent-fix
# Merge and deploy
```

---

## ✅ Current Status

- ✅ Git repository configured
- ✅ GitHub SSH key set up
- ✅ Remote origin configured
- ✅ CI/CD auto-deploy active
- ✅ Branch protection (recommended)
- ✅ Security hardening complete

## 🎯 Next Steps

1. Implement semantic versioning for releases
2. Add automated testing to CI/CD
3. Set up branch protection rules on GitHub
4. Configure pre-commit hooks for quality checks
5. Create comprehensive documentation for all services

---

**Remember**: OpenClaw is now a professional Coding Factory. Follow these standards, keep commits atomic, document everything, and always pull before pushing! 🏭✨