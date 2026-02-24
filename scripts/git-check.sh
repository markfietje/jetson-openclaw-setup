#!/bin/bash
# Pre-commit Quality Checker for Coding Factory
# Validates code quality before committing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
MAX_FILE_SIZE=$((5 * 1024 * 1024))  # 5MB
MAX_LINE_LENGTH=120
CHECK_PATTERNS=true
DEBUG=true

# Statistics
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

# Functions
print_header() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Pre-commit Quality Checker - Coding Factory${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
    ((PASSED_CHECKS++))
    ((TOTAL_CHECKS++))
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
    ((WARNINGS++))
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
    ((FAILED_CHECKS++))
    ((TOTAL_CHECKS++))
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

print_debug() {
    if [ "$DEBUG" = true ]; then
        echo -e "${BLUE}  DEBUG: $1${NC}"
    fi
}

# Get staged files
get_staged_files() {
    git diff --cached --name-only --diff-filter=ACM
}

# Check 1: File size validation
check_file_sizes() {
    print_info "Checking file sizes..."
    local found_large_files=false

    while IFS= read -r file; do
        if [ -f "$file" ]; then
            local size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
            if [ "$size" -gt "$MAX_FILE_SIZE" ]; then
                local size_mb=$((size / 1024 / 1024))
                print_warning "Large file: $file (${size_mb}MB > ${MAX_FILE_SIZE}MB)"
                found_large_files=true
            fi
        fi
    done < <(get_staged_files)

    if [ "$found_large_files" = false ]; then
        print_success "No large files detected"
    fi
}

# Check 2: Sensitive data detection
check_sensitive_data() {
    if [ "$CHECK_PATTERNS" = false ]; then
        return
    fi

    print_info "Checking for sensitive data..."
    local found_issues=false
    local patterns=(
        "password\s*[=:]\s*['\"]?\w+"
        "api[_-]?key\s*[=:]\s*['\"]?\w+"
        "secret\s*[=:]\s*['\"]?\w+"
        "token\s*[=:]\s*['\"]?\w+"
        "private[_-]?key"
        "aws[_-]?access[_-]?key"
        "aws[_-]?secret"
        "BEGIN\s+PRIVATE\s+KEY"
        "BEGIN\s+RSA\s+PRIVATE\s+KEY"
    )

    while IFS= read -r file; do
        if [ -f "$file" ] && [ -r "$file" ]; then
            for pattern in "${patterns[@]}"; do
                if grep -qiE "$pattern" "$file" 2>/dev/null; then
                    print_warning "Potential sensitive data in: $file"
                    print_debug "Pattern matched: $pattern"
                    found_issues=true
                    break
                fi
            done
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No sensitive data detected"
    fi
}

# Check 3: Debugging statements
check_debug_statements() {
    if [ "$CHECK_PATTERNS" = false ]; then
        return
    fi

    print_info "Checking for debugging statements..."
    local found_issues=false
    declare -A debug_patterns=(
        ["\.rs$"]="println!|eprintln!|dbg!"
        ["\.py$"]="print\(|pprint\(|pdb\.set_trace"
        ["\.js$"]="console\.log|console\.debug|console\.warn"
        ["\.ts$"]="console\.log|console\.debug|console\.warn"
        ["\.sh$"]="set -x|set -v"
    )

    while IFS= read -r file; do
        if [ -f "$file" ] && [ -r "$file" ]; then
            for pattern in "${!debug_patterns[@]}"; do
                if [[ "$file" =~ $pattern ]]; then
                    if grep -qE "${debug_patterns[$pattern]}" "$file" 2>/dev/null; then
                        print_warning "Debug statements in: $file"
                        print_debug "Pattern: ${debug_patterns[$pattern]}"
                        found_issues=true
                        break
                    fi
                fi
            done
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No debugging statements detected"
    fi
}

# Check 4: TODO/FIXME comments
check_todo_comments() {
    if [ "$CHECK_PATTERNS" = false ]; then
        return
    fi

    print_info "Checking for TODO/FIXME comments..."
    local found_issues=false

    while IFS= read -r file; do
        if [ -f "$file" ] && [ -r "$file" ]; then
            if grep -qE "TODO|FIXME|HACK|XXX" "$file" 2>/dev/null; then
                print_warning "TODO/FIXME comments in: $file"
                found_issues=true
            fi
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No TODO/FIXME comments detected"
    fi
}

# Check 5: Line length (for certain file types)
check_line_length() {
    print_info "Checking line lengths..."
    local found_issues=false
    local long_files=()

    while IFS= read -r file; do
        if [ -f "$file" ] && [ -r "$file" ]; then
            # Check only text files and certain extensions
            case "$file" in
                *.rs|*.py|*.js|*.ts|*.sh|*.md|*.txt|*.toml|*.yaml|*.yml)
                    local long_lines=$(awk "length > $MAX_LINE_LENGTH" "$file" 2>/dev/null | wc -l)
                    if [ "$long_lines" -gt 0 ]; then
                        long_files+=("$file: $long_lines lines > ${MAX_LINE_LENGTH} chars")
                        found_issues=true
                    fi
                    ;;
            esac
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "Line length checks passed"
    else
        for file_info in "${long_files[@]}"; do
            print_warning "$file_info"
        done
    fi
}

# Check 6: Binary files
check_binary_files() {
    print_info "Checking for unintended binary files..."
    local found_issues=false
    local binary_extensions=(
        ".exe" ".dll" ".so" ".dylib" ".bin" ".dat"
        ".db" ".db-shm" ".db-wal"
        ".pem" ".key" ".crt"
        ".zip" ".tar" ".gz" ".rar"
        ".png" ".jpg" ".jpeg" ".gif" ".bmp" ".ico"
    )

    while IFS= read -r file; do
        if [ -f "$file" ]; then
            for ext in "${binary_extensions[@]}"; do
                if [[ "$file" == *"$ext" ]]; then
                    print_warning "Binary file staged: $file"
                    print_debug "Ensure this should be committed"
                    found_issues=true
                    break
                fi
            done
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No unintended binary files"
    fi
}

# Check 7: File permissions
check_file_permissions() {
    print_info "Checking file permissions..."
    local found_issues=false

    while IFS= read -r file; do
        if [ -f "$file" ]; then
            local perms=$(stat -f%A "$file" 2>/dev/null || stat -c%a "$file" 2>/dev/null || echo "644")

            # Check for executable files that shouldn't be
            if [[ "$perms" =~ ^[7567][7567][7567]$ ]] && ! [[ "$file" =~ \.(sh|py|rb)$ ]]; then
                print_warning "Executable permissions: $file ($perms)"
                print_debug "Consider: chmod 644 $file"
                found_issues=true
            fi
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "File permissions look good"
    fi
}

# Check 8: Common mistakes
check_common_mistakes() {
    if [ "$CHECK_PATTERNS" = false ]; then
        return
    fi

    print_info "Checking for common mistakes..."
    local found_issues=false

    while IFS= read -r file; do
        if [ -f "$file" ] && [ -r "$file" ]; then
            # Check for trailing whitespace
            if grep -q " $" "$file" 2>/dev/null; then
                print_warning "Trailing whitespace in: $file"
                found_issues=true
            fi

            # Check for mixed line endings
            if file "$file" | grep -q "CRLF" 2>/dev/null; then
                print_warning "Mixed line endings (CRLF) in: $file"
                print_debug "Run: dos2unix $file"
                found_issues=true
            fi

            # Check for tabs in certain files
            if [[ "$file" =~ \.(py|rs|js|ts)$ ]] && grep -q $'\t' "$file" 2>/dev/null; then
                print_warning "Tabs in: $file (use spaces)"
                found_issues=true
            fi
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No common mistakes detected"
    fi
}

# Check 9: Git-specific checks
check_git_specific() {
    print_info "Checking Git-specific issues..."
    local found_issues=false

    # Check for .gitignore violations
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            # Check if file should be gitignored
            if git check-ignore -q "$file" 2>/dev/null; then
                print_warning "File matches .gitignore: $file"
                found_issues=true
            fi
        fi
    done < <(get_staged_files)

    if [ "$found_issues" = false ]; then
        print_success "No Git-specific issues"
    fi
}

# Check 10: Language-specific checks
check_language_specific() {
    print_info "Running language-specific checks..."

    while IFS= read -r file; do
        if [ ! -f "$file" ] || [ ! -r "$file" ]; then
            continue
        fi

        case "$file" in
            *.rs)
                # Rust syntax check
                if command -v rustc &> /dev/null; then
                    if rustc --crate-type lib "$file" &> /dev/null; then
                        print_debug "Rust syntax OK: $file"
                    else
                        print_warning "Rust syntax check failed: $file"
                    fi
                fi
                ;;
            *.py)
                # Python syntax check
                if command -v python3 &> /dev/null; then
                    if python3 -m py_compile "$file" 2>/dev/null; then
                        print_debug "Python syntax OK: $file"
                    else
                        print_warning "Python syntax check failed: $file"
                    fi
                fi
                ;;
            *.sh)
                # Shell script check
                if command -v shellcheck &> /dev/null; then
                    if shellcheck "$file" &> /dev/null; then
                        print_debug "Shell script OK: $file"
                    else
                        print_warning "Shell script issues: $file"
                    fi
                fi
                ;;
        esac
    done < <(get_staged_files)

    print_success "Language-specific checks completed"
}

# Show summary
show_summary() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Summary${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "Total checks: ${BLUE}${TOTAL_CHECKS}${NC}"
    echo -e "Passed:       ${GREEN}${PASSED_CHECKS}${NC}"
    echo -e "Warnings:     ${YELLOW}${WARNINGS}${NC}"
    echo -e "Failed:       ${RED}${FAILED_CHECKS}${NC}"
    echo ""

    if [ $FAILED_CHECKS -eq 0 ]; then
        print_success "Pre-commit checks passed! You can commit your changes."
        exit 0
    else
        print_error "Pre-commit checks failed! Please fix the issues before committing."
        exit 1
    fi
}

# Main execution
main() {
    print_header

    # Check if there are staged files
    if [ -z "$(get_staged_files)" ]; then
        print_warning "No staged files found"
        print_info "Stage files with: git add <files>"
        exit 0
    fi

    print_info "Found staged files:"
    get_staged_files | nl
    echo ""

    # Run all checks
    check_file_sizes
    check_sensitive_data
    check_debug_statements
    check_todo_comments
    check_line_length
    check_binary_files
    check_file_permissions
    check_common_mistakes
    check_git_specific
    check_language_specific

    # Show summary
    show_summary
}

# Run main function
main
