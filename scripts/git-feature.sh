#!/bin/bash
# Git Feature Branch Creator for Coding Factory
# Creates feature branches following best practices

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_DIR="${1:-$(pwd)}"
FEATURE_TYPE="${2:-feature}"
FEATURE_NAME=""

# Functions
print_header() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Feature Branch Creator - Coding Factory${NC}"
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

# Show usage
show_usage() {
    echo "Usage: $0 [directory] [branch_type]"
    echo ""
    echo "Arguments:"
    echo "  directory    Path to git repository (default: current directory)"
    echo "  branch_type  Type of branch: feature, fix, hotfix, refactor, docs, test, chore"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Interactive mode in current dir"
    echo "  $0 ~/openclaw-repo/ feature          # Create feature branch"
    echo "  $0 ~/openclaw-repo/ fix              # Create fix branch"
    echo "  $0 ~/openclaw-repo/ hotfix           # Create hotfix branch"
    echo ""
    echo "Branch Naming Convention:"
    echo "  feature/description      -> New features"
    echo "  fix/description          -> Bug fixes"
    echo "  hotfix/description       -> Urgent production fixes"
    echo "  refactor/description     -> Code refactoring"
    echo "  docs/description         -> Documentation changes"
    echo "  test/description         -> Test additions/changes"
    echo "  chore/description        -> Maintenance tasks"
    echo ""
}

# Check if we're in a git repository
check_git_repo() {
    if ! git -C "$REPO_DIR" rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not a git repository: $REPO_DIR"
        exit 1
    fi
    print_success "Git repository found: $REPO_DIR"
}

# Get current branch
get_current_branch() {
    git -C "$REPO_DIR" branch --show-current
}

# Check for uncommitted changes
check_uncommitted_changes() {
    print_info "Checking for uncommitted changes..."

    if ! git -C "$REPO_DIR" diff-index --quiet HEAD -- 2>/dev/null; then
        print_warning "You have uncommitted changes!"
        echo ""
        git -C "$REPO_DIR" status --short
        echo ""
        print_warning "Please commit or stash changes before creating a feature branch"
        return 1
    fi

    print_success "Working tree is clean"
    return 0
}

# Validate branch type
validate_branch_type() {
    case "$FEATURE_TYPE" in
        feature|fix|hotfix|refactor|docs|test|chore)
            print_success "Branch type: $FEATURE_TYPE"
            return 0
            ;;
        *)
            print_error "Invalid branch type: $FEATURE_TYPE"
            echo ""
            print_info "Valid types: feature, fix, hotfix, refactor, docs, test, chore"
            return 1
            ;;
    esac
}

# Get feature name from user
get_feature_name() {
    local current_branch=$(get_current_branch)

    echo ""
    print_info "Creating $FEATURE_TYPE branch from: $current_branch"
    echo ""

    # Prompt for feature name
    while [ -z "$FEATURE_NAME" ]; do
        read -p "Enter feature name (e.g., webhook-support, api-v2, memory-leak): " FEATURE_NAME

        if [ -z "$FEATURE_NAME" ]; then
            print_warning "Feature name cannot be empty"
        fi
    done

    # Clean the feature name
    # Convert to lowercase
    FEATURE_NAME=$(echo "$FEATURE_NAME" | tr '[:upper:]' '[:lower:]')

    # Replace spaces with hyphens
    FEATURE_NAME=$(echo "$FEATURE_NAME" | tr ' ' '-')

    # Remove special characters (keep only alphanumeric, hyphens)
    FEATURE_NAME=$(echo "$FEATURE_NAME" | sed 's/[^a-z0-9-]//g')

    # Remove leading/trailing hyphens
    FEATURE_NAME=$(echo "$FEATURE_NAME" | sed 's/^-*//;s/-*$//')

    if [ -z "$FEATURE_NAME" ]; then
        print_error "Invalid feature name after cleaning"
        exit 1
    fi

    print_success "Feature name: $FEATURE_NAME"
}

# Check if branch already exists
check_branch_exists() {
    local branch_name="$FEATURE_TYPE/$FEATURE_NAME"

    if git -C "$REPO_DIR" show-ref --verify --quiet "refs/heads/$branch_name" 2>/dev/null; then
        print_warning "Branch '$branch_name' already exists!"
        echo ""
        print_info "Options:"
        echo "  1. Switch to existing branch: git checkout $branch_name"
        echo "  2. Delete existing branch: git branch -D $branch_name"
        echo "  3. Choose a different name"
        exit 1
    fi
}

# Pull latest changes
pull_latest() {
    print_info "Pulling latest changes from origin/main..."

    if git -C "$REPO_DIR" pull origin main 2>&1 | grep -q "fatal"; then
        print_warning "Failed to pull from origin/main (might not exist)"
        print_info "Continuing with current branch state..."
    else
        print_success "Pulled latest changes"
    fi
}

# Create feature branch
create_feature_branch() {
    local branch_name="$FEATURE_TYPE/$FEATURE_NAME"

    print_info "Creating feature branch: $branch_name"

    if git -C "$REPO_DIR" checkout -b "$branch_name"; then
        print_success "Feature branch created: $branch_name"
    else
        print_error "Failed to create feature branch"
        exit 1
    fi
}

# Show next steps
show_next_steps() {
    local branch_name="$FEATURE_TYPE/$FEATURE_NAME"

    echo ""
    print_success "Feature branch setup complete!"
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Next Steps${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    print_info "1. Start development:"
    echo "   cd $REPO_DIR"
    echo "   # ... make changes ..."
    echo ""
    print_info "2. Commit changes with conventional commits:"
    echo "   git add ."
    echo "   git commit -m \"feat: description of changes\""
    echo ""
    print_info "3. Push to GitHub:"
    echo "   git push -u origin $branch_name"
    echo ""
    print_info "4. Create pull request (if needed):"
    echo "   gh pr create --title \"$FEATURE_TYPE: $FEATURE_NAME\" \\"
    echo "     --body \"Description of changes\""
    echo ""
    print_info "5. Merge to main when ready:"
    echo "   git checkout main"
    echo "   git pull origin main"
    echo "   git merge $branch_name"
    echo "   git push origin main"
    echo ""
    print_info "6. Delete feature branch (optional):"
    echo "   git branch -d $branch_name"
    echo "   git push origin --delete $branch_name"
    echo ""

    # Show commit message guide
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Commit Message Guide${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Format: <type>[optional scope]: <description>"
    echo ""
    echo "Types:"
    echo "  feat     - New feature"
    echo "  fix      - Bug fix"
    echo "  docs     - Documentation changes"
    echo "  style    - Code style changes"
    echo "  refactor - Code refactoring"
    echo "  test     - Test additions/changes"
    echo "  chore    - Maintenance tasks"
    echo ""
    echo "Examples:"
    echo "  feat: add webhook endpoint for signal processing"
    echo "  fix: resolve memory leak in brain-server"
    echo "  docs: update API documentation"
    echo "  refactor: simplify error handling"
    echo ""
}

# Main execution
main() {
    print_header
    print_info "Repository: $REPO_DIR"
    print_info "Branch Type: $FEATURE_TYPE"
    echo ""

    # Check git repository
    check_git_repo
    echo ""

    # Validate branch type
    if ! validate_branch_type; then
        show_usage
        exit 1
    fi
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

    # Get feature name
    get_feature_name
    echo ""

    # Check if branch exists
    check_branch_exists
    echo ""

    # Pull latest changes
    pull_latest
    echo ""

    # Create feature branch
    create_feature_branch
    echo ""

    # Show next steps
    show_next_steps
}

# Handle help flag
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    show_usage
    exit 0
fi

# Run main function
main
