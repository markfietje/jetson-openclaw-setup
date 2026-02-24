#!/bin/bash
# Git Sync Utility for Coding Factory
# Ensures repositories are synchronized before development work

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_DIR="${1:-$(pwd)}"
BRANCH="${2:-main}"

# Functions
print_header() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Git Sync - Coding Factory Workflow${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Check if we're in a git repository
check_git_repo() {
    if ! git -C "$REPO_DIR" rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not a git repository: $REPO_DIR"
        exit 1
    fi
    print_success "Git repository found: $REPO_DIR"
}

# Check for uncommitted changes
check_uncommitted_changes() {
    print_info "Checking for uncommitted changes..."

    if ! git -C "$REPO_DIR" diff-index --quiet HEAD -- 2>/dev/null; then
        print_warning "You have uncommitted changes!"
        echo ""
        git -C "$REPO_DIR" status --short
        echo ""
        print_warning "Please commit or stash changes before syncing"
        return 1
    fi

    print_success "No uncommitted changes"
    return 0
}

# Fetch remote updates
fetch_remote() {
    print_info "Fetching remote updates..."

    if git -C "$REPO_DIR" fetch origin "$BRANCH" 2>&1 | grep -q "fatal"; then
        print_error "Failed to fetch from remote"
        return 1
    fi

    print_success "Fetched remote updates"
    return 0
}

# Check if branch is up to date
check_sync_status() {
    print_info "Checking sync status..."

    LOCAL=$(git -C "$REPO_DIR" rev-parse HEAD)
    REMOTE=$(git -C "$REPO_DIR" rev-parse origin/"$BRANCH")

    if [ "$LOCAL" = "$REMOTE" ]; then
        print_success "Already up to date"
        return 0
    else
        print_warning "Local and remote differ"
        return 1
    fi
}

# Show what's new
show_changes() {
    print_info "Changes that will be applied:"
    echo ""
    git -C "$REPO_DIR" log --oneline HEAD..origin/"$BRANCH" | head -10
    echo ""
}

# Pull changes
pull_changes() {
    print_info "Pulling changes from origin/$BRANCH..."

    # Check if there are conflicts
    if git -C "$REPO_DIR" merge-base --is-ancestor origin/"$BRANCH" HEAD 2>/dev/null; then
        # We're ahead, fast-forward possible
        git -C "$REPO_DIR" merge --ff-only origin/"$BRANCH"
    elif git -C "$REPO_DIR" merge-base --is-ancestor HEAD origin/"$BRANCH" 2>/dev/null; then
        # We're behind, rebase for clean history
        print_info "Rebasing local changes..."
        git -C "$REPO_DIR" rebase origin/"$BRANCH"
    else
        # Divergent branches, merge
        print_warning "Branches have diverged, merging..."
        git -C "$REPO_DIR" merge origin/"$BRANCH" --no-edit
    fi

    print_success "Successfully pulled changes"
}

# Show final status
show_status() {
    echo ""
    print_info "Current status:"
    echo ""
    git -C "$REPO_DIR" status
    echo ""

    # Show recent commits
    print_info "Recent commits:"
    echo ""
    git -C "$REPO_DIR" log --oneline -5
    echo ""
}

# Main execution
main() {
    print_header
    print_info "Repository: $REPO_DIR"
    print_info "Branch: $BRANCH"
    echo ""

    # Check git repository
    check_git_repo
    echo ""

    # Check for uncommitted changes
    if ! check_uncommitted_changes; then
        echo ""
        print_info "Options:"
        echo "  1. Commit changes: git add . && git commit -m 'your message'"
        echo "  2. Stash changes: git stash"
        echo "  3. Discard changes: git reset --hard HEAD"
        exit 1
    fi
    echo ""

    # Fetch remote
    if ! fetch_remote; then
        exit 1
    fi
    echo ""

    # Check sync status
    if check_sync_status; then
        show_status
        exit 0
    fi
    echo ""

    # Show what's coming
    show_changes

    # Confirm pull
    read -p "Do you want to pull these changes? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_warning "Pull cancelled"
        exit 0
    fi
    echo ""

    # Pull changes
    if pull_changes; then
        show_status
        print_success "Repository synchronized successfully!"
        echo ""
        print_info "You can now start development work"
        exit 0
    else
        print_error "Failed to pull changes"
        print_info "You may need to resolve conflicts manually"
        exit 1
    fi
}

# Run main function
main
