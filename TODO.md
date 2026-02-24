# TODO - GitHub Actions Automated Deployment Setup

## 🎯 Goal: Enable Fully Automated Deployment to Jetson

This document outlines the steps needed to set up GitHub Actions for automated deployment of brain-server and signal-gateway to the Jetson Nano.

---

## 📋 Prerequisites

- ✅ Private GitHub repository created
- ✅ SSH access to Jetson Nano (jetson@jetson)
- ✅ Automated deployment workflows created in `.github/workflows/`

---

## 🔐 Step 1: Generate SSH Key for GitHub Actions

**Run these commands on your local machine (MacBook):**

```bash
# 1. Generate new SSH key pair for GitHub Actions
ssh-keygen -t ed25519 -C "github-actions@jetson" -f ~/.ssh/github_actions_jetson

# 2. Copy public key to Jetson
ssh-copy-id -i ~/.ssh/github_actions_jetson.pub jetson@jetson

# 3. Test SSH connection
ssh -i ~/.ssh/github_actions_jetson jetson@jetson "echo 'SSH works!'"

# 4. Get Jetson's SSH host key (for known_hosts)
ssh-keyscan jetson > ~/.ssh/jetson_known_hosts

# 5. View the contents of both files
cat ~/.ssh/github_actions_jetson
cat ~/.ssh/jetson_known_hosts
```

---

## 🔑 Step 2: Add Secrets to GitHub

Go to: **GitHub Repository → Settings → Secrets and variables → Actions → New repository secret**

### Secret 1: JETSON_SSH_KEY
- **Name:** `JETSON_SSH_KEY`
- **Value:** Contents of `~/.ssh/github_actions_jetson` (the PRIVATE key)
- **How to get it:** `cat ~/.ssh/github_actions_jetson`

### Secret 2: JETSON_SSH_KNOWN_HOSTS
- **Name:** `JETSON_SSH_KNOWN_HOSTS`
- **Value:** Contents of `~/.ssh/jetson_known_hosts`
- **How to get it:** `cat ~/.ssh/jetson_known_hosts`

---

## ✅ Step 3: Test Automated Deployment

After adding the secrets, test the automated deployment:

### Option A: Push a Test Commit
```bash
git commit -m "test: trigger automated deployment" --allow-empty
git push origin main
```

### Option B: Manual Trigger
1. Go to: **GitHub → Actions → Auto Deploy to Jetson**
2. Click **"Run workflow"**
3. Select which services to deploy
4. Click **"Run workflow"** button

---

## 🎉 Step 4: Verify Deployment

After the workflow completes, verify on Jetson:

```bash
# Check service status
ssh jetson@jetson "sudo systemctl status brain-server"
ssh jetson@jetson "sudo systemctl status signal-gateway"

# Test API endpoints
ssh jetson@jetson "curl -s http://localhost:8765/health | jq ."
ssh jetson@jetson "curl -s http://localhost:8080/v1/health | jq ."
```

---

## 📝 How It Works (Once Setup)

After setup, just push code and GitHub does the rest:

```bash
git add .
git commit -m "feat: amazing new feature"
git push origin main
```

**GitHub Actions automatically:**
1. ✅ Builds ARM64 binaries for both services
2. ✅ Uploads to Jetson via SSH
3. ✅ Restarts systemd services
4. ✅ Runs health checks
5. ✅ Reports deployment status

---

## 🛠️ Troubleshooting

### SSH Connection Issues
```bash
# Test SSH from local machine
ssh -i ~/.ssh/github_actions_jetson jetson@jetson "echo 'Test successful'"
```

### Permission Issues on Jetson
```bash
# Ensure jetson user can restart services
sudo usermod -aG systemd-journal jetson
```

### Deployment Failures
- Check GitHub Actions logs: **Actions → Auto Deploy to Jetson → Click on workflow run**
- Check Jetson service logs: `ssh jetson@jetson "sudo journalctl -u brain-server -f"`

---

## 📚 Additional Resources

- **GitHub Actions Workflow:** `.github/workflows/auto-deploy.yml`
- **CI/CD Documentation:** `.github/workflows/README.md`
- **Development Rules:** `.rules`
- **Service Documentation:** `README.md`

---

## ✨ Completion Checklist

- [ ] Generate SSH key pair for GitHub Actions
- [ ] Copy public key to Jetson
- [ ] Add JETSON_SSH_KEY secret to GitHub
- [ ] Add JETSON_SSH_KNOWN_HOSTS secret to GitHub
- [ ] Test automated deployment (push commit or manual trigger)
- [ ] Verify services are running on Jetson
- [ ] Verify API endpoints are responding
- [ ] Celebrate! 🎉

---

**Last Updated:** 2026-02-23
**Status:** ⏳ Pending Setup