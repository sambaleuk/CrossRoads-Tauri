#!/bin/bash
#
# XRoads/Nexus Common Library
# Shared functions and variables for all loop scripts
# Portable version - no hardcoded paths
#

# ============================================================================
# COLORS
# ============================================================================
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export MAGENTA='\033[0;35m'
export DIM='\033[2m'
export BOLD='\033[1m'
export NC='\033[0m'

# ============================================================================
# CONFIGURATION
# ============================================================================
export NEXUS_VERSION="2.1.0"
# NEXUS_HOME can be overridden, defaults to ~/.nexus for compatibility
export NEXUS_HOME="${NEXUS_HOME:-$HOME/.nexus}"
export NEXUS_TEMPLATES="${NEXUS_HOME}/templates"

# Default file names
export PRD_FILE="prd.json"
export PRD_TESTS_FILE="prd-tests.json"
export PROGRESS_FILE="progress.txt"
export TEST_PROGRESS_FILE="test-progress.json"
export TEST_RESULTS_FILE="test-results.json"
export AGENTS_FILE="AGENT.md"

# ============================================================================
# LOGGING
# ============================================================================
log_info() {
    echo -e "${BLUE}[Nexus]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_step() {
    echo -e "${CYAN}[${1}]${NC} $2"
}

# ============================================================================
# DEPENDENCY CHECKS
# ============================================================================
check_claude() {
    if ! command -v claude &> /dev/null; then
        log_error "Claude Code CLI not found"
        echo "Install with: npm install -g @anthropic-ai/claude-code"
        return 1
    fi
    return 0
}

check_gemini() {
    if ! command -v gemini &> /dev/null; then
        log_error "Gemini CLI not found"
        echo "Install with: npm install -g @anthropic-ai/gemini-cli"
        return 1
    fi
    return 0
}

check_codex() {
    if ! command -v codex &> /dev/null; then
        log_error "Codex CLI not found"
        echo "Install Codex CLI from OpenAI"
        return 1
    fi
    return 0
}

check_jq() {
    if ! command -v jq &> /dev/null; then
        log_error "jq not found"
        echo "Install with: brew install jq (macOS) or apt install jq (Linux)"
        return 1
    fi
    return 0
}

check_git() {
    if ! command -v git &> /dev/null; then
        log_error "git not found"
        return 1
    fi
    return 0
}

check_all_deps() {
    local failed=0
    check_claude || failed=1
    check_jq || failed=1
    check_git || failed=1
    return $failed
}

check_gemini_deps() {
    local failed=0
    check_gemini || failed=1
    check_jq || failed=1
    check_git || failed=1
    return $failed
}

check_codex_deps() {
    local failed=0
    check_codex || failed=1
    check_jq || failed=1
    check_git || failed=1
    return $failed
}

# ============================================================================
# JSON HELPERS
# ============================================================================
json_get() {
    local file="$1"
    local path="$2"
    local default="${3:-}"

    if [ -f "$file" ]; then
        local result=$(jq -r "$path // \"$default\"" "$file" 2>/dev/null)
        if [ "$result" = "null" ]; then
            echo "$default"
        else
            echo "$result"
        fi
    else
        echo "$default"
    fi
}

json_set() {
    local file="$1"
    local path="$2"
    local value="$3"

    local tmp_file=$(mktemp)
    jq "$path = $value" "$file" > "$tmp_file" && mv "$tmp_file" "$file"
}

# ============================================================================
# PRD HELPERS
# ============================================================================
prd_count_pending() {
    local file="${1:-$PRD_FILE}"
    json_get "$file" '[.user_stories[] | select(.status != "complete")] | length' "0"
}

prd_count_total() {
    local file="${1:-$PRD_FILE}"
    json_get "$file" '[.user_stories[]] | length' "0"
}

prd_get_feature_name() {
    local file="${1:-$PRD_FILE}"
    json_get "$file" '.feature_name' "Unknown Feature"
}

# PRD Tests helpers
prd_tests_count_pending() {
    local file="${1:-$PRD_TESTS_FILE}"
    json_get "$file" '[.test_suites[].test_cases[] | select(.status == "pending")] | length' "0"
}

prd_tests_count_total() {
    local file="${1:-$PRD_TESTS_FILE}"
    json_get "$file" '[.test_suites[].test_cases[]] | length' "0"
}

prd_tests_get_project_name() {
    local file="${1:-$PRD_TESTS_FILE}"
    json_get "$file" '.project.name' "Unknown Project"
}

# ============================================================================
# PRD PROTECTION
# ============================================================================
# Some agents (e.g., Gemini with create-next-app) delete all files in the
# worktree directory, including prd.json/progress.txt/AGENT.md. We backup
# these critical files before each iteration and restore if they vanish.

backup_loop_files() {
    local backup_dir=".xroads-backup"
    mkdir -p "$backup_dir"
    for f in "$PRD_FILE" "$PROGRESS_FILE" "$AGENTS_FILE"; do
        if [[ -f "$f" ]]; then
            cp "$f" "$backup_dir/"
        fi
    done
}

restore_loop_files() {
    local backup_dir=".xroads-backup"
    [[ -d "$backup_dir" ]] || return 0
    local restored=0
    for f in "$PRD_FILE" "$PROGRESS_FILE" "$AGENTS_FILE"; do
        if [[ ! -f "$f" && -f "$backup_dir/$f" ]]; then
            cp "$backup_dir/$f" "$f"
            restored=$((restored + 1))
        fi
    done
    if [[ $restored -gt 0 ]]; then
        log_warn "Restored $restored loop files deleted by agent"
    fi
}

# ============================================================================
# STATUS FILE SYNC
# ============================================================================
# Sync completed stories from local prd.json to the central status.json
# This is critical because agents may be sandboxed and unable to write
# to the central status file directly (e.g., Gemini MCP filesystem).
# The loop script runs without sandbox, so it can always write.
sync_prd_to_status() {
    local status_file="${CROSSROADS_STATUS_FILE:-}"
    local prd_file="${1:-$PRD_FILE}"

    # Skip if no status file configured (not an XRoads orchestrated run)
    if [[ -z "$status_file" || ! -f "$status_file" ]]; then
        return 0
    fi

    if [[ ! -f "$prd_file" ]]; then
        return 0
    fi

    # Get completed story IDs from local prd.json
    local completed_ids
    completed_ids=$(jq -r '.user_stories[] | select(.status == "complete") | .id' "$prd_file" 2>/dev/null)

    if [[ -z "$completed_ids" ]]; then
        return 0
    fi

    local synced=0
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    while IFS= read -r story_id; do
        # Check if already marked complete in status.json
        local current_status
        current_status=$(jq -r ".stories[\"$story_id\"].status // \"unknown\"" "$status_file" 2>/dev/null)

        if [[ "$current_status" != "complete" && "$current_status" != "unknown" ]]; then
            local tmp_file
            tmp_file=$(mktemp /tmp/status_sync.XXXXXX)
            jq --arg id "$story_id" --arg ts "$timestamp" \
              '.stories[$id].status = "complete" | .stories[$id].completedAt = $ts | .updatedAt = $ts' \
              "$status_file" > "$tmp_file" && mv "$tmp_file" "$status_file"
            synced=$((synced + 1))
        fi
    done <<< "$completed_ids"

    if [[ $synced -gt 0 ]]; then
        log_info "Synced $synced completed stories to status.json"
    fi
}

# ============================================================================
# RATE-LIMIT DETECTION
# ============================================================================
# Detect API rate limiting (429) in agent output logs. When detected, the loop
# should apply a longer cooldown instead of burning iterations with 3s retries.
# Gemini CLI retries 5x internally, so by the time we see exit code != 0 with
# 429 in the log, the API has been hammered for minutes already.

RATE_LIMIT_COOLDOWN=${RATE_LIMIT_COOLDOWN:-60}  # seconds to wait after rate limit
RATE_LIMIT_FAILOVER_EXIT=42                     # special exit code: request agent failover
RATE_LIMIT_FAILOVER_THRESHOLD=${RATE_LIMIT_FAILOVER_THRESHOLD:-1}  # rate limits before failover (1 = immediate)

detect_rate_limit() {
    local log_file="$1"
    [[ -f "$log_file" ]] || return 1

    # Check for common rate-limit patterns across all 3 agents
    if grep -qi '429\|RESOURCE_EXHAUSTED\|rate.limit\|Too Many Requests\|quota.*exceeded' "$log_file" 2>/dev/null; then
        return 0  # rate limit detected
    fi
    return 1  # no rate limit
}

handle_rate_limit() {
    local cooldown="${1:-$RATE_LIMIT_COOLDOWN}"
    log_warn "API rate limit detected (429). Cooling down for ${cooldown}s..."
    log_info "Tip: set RATE_LIMIT_COOLDOWN=120 to adjust cooldown duration"
    sleep "$cooldown"
}

# ============================================================================
# AUTO-COMMIT
# ============================================================================
# After each iteration, commit any uncommitted changes the agent left behind.
# Some agents (e.g., Gemini) may implement code but forget to git commit.
# Loop files (prd.json, progress.txt, AGENT.md) are .gitignored so they
# won't pollute the commit.

auto_commit_if_needed() {
    # Check if we're in a git repo
    if ! git rev-parse --is-inside-work-tree &>/dev/null; then
        return 0
    fi

    # Check for uncommitted changes (tracked files only)
    if git diff --quiet HEAD 2>/dev/null && git diff --cached --quiet HEAD 2>/dev/null; then
        # No changes to tracked files, check for untracked files (excluding loop files)
        local untracked
        untracked=$(git ls-files --others --exclude-standard 2>/dev/null | head -1)
        if [[ -z "$untracked" ]]; then
            return 0
        fi
    fi

    # Stage all changes (respects .gitignore)
    git add -A 2>/dev/null || return 0

    # Check if there's actually anything staged
    if git diff --cached --quiet HEAD 2>/dev/null; then
        return 0
    fi

    # Get current story ID from prd.json for commit message
    local story_id="unknown"
    if [[ -f "$PRD_FILE" ]]; then
        story_id=$(jq -r '[.user_stories[] | select(.status == "complete")] | last | .id // "WIP"' "$PRD_FILE" 2>/dev/null || echo "WIP")
    fi

    # Commit with descriptive message
    git commit -m "feat: auto-commit after iteration ($story_id)" --no-verify 2>/dev/null
    if [[ $? -eq 0 ]]; then
        log_info "Auto-committed changes (story: $story_id)"
    fi
}

# ============================================================================
# BANNER
# ============================================================================
show_nexus_banner() {
    local agent="${1:-}"
    local agent_color="${2:-$CYAN}"

    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}     ${BLUE}╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝${NC}              ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}                     ${MAGENTA}L O O P${NC}                                 ${CYAN}║${NC}"
    if [[ -n "$agent" ]]; then
        echo -e "${CYAN}║${NC}                 ${DIM}───${NC} ${agent_color}${agent}${NC} ${DIM}───${NC}                            ${CYAN}║${NC}"
    fi
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

# ============================================================================
# ITERATION DISPLAY
# ============================================================================
show_iteration_header() {
    local current="$1"
    local max="$2"
    local complete="$3"
    local total="$4"
    local remaining=$((total - complete))
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  Iteration $current of $max  │  ${DIM}$timestamp${NC}"
    echo -e "${CYAN}  Progress: ${GREEN}$complete${NC}/${total} (${RED}$remaining remaining${NC})"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

show_completion_banner() {
    local iterations="$1"
    local feature="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  ✅ COMPLETE after $iterations iterations!${NC}"
    echo -e "${GREEN}  Feature: ${YELLOW}$feature${NC}"
    echo -e "${GREEN}  Completed at: ${DIM}$timestamp${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

show_timeout_banner() {
    local max="$1"
    local remaining="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    echo ""
    echo -e "${RED}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${RED}  Reached max iterations ($max)${NC}"
    echo -e "${RED}  $remaining items still pending${NC}"
    echo -e "${RED}  Stopped at: ${DIM}$timestamp${NC}"
    echo -e "${RED}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

# ============================================================================
# FILE TEMPLATES
# ============================================================================
create_progress_file() {
    local feature="${1:-New Feature}"
    cat << EOF
# Nexus Loop Progress Log
## Feature: $feature
## Started: $(date '+%Y-%m-%d %H:%M:%S')

## Learnings
<!-- Patterns discovered, gotchas, reusable knowledge -->

---

## Session Log

EOF
}

create_agents_file() {
    cat << 'EOF'
# Codebase Patterns

This file contains reusable patterns for AI agents working on this codebase.

## Architecture

<!-- Add architecture patterns here -->

## Code Style

<!-- Add code style patterns here -->

## Testing

<!-- Add testing patterns here -->

EOF
}

create_test_progress_file() {
    cat << 'EOF'
{
  "started_at": null,
  "last_updated": null,
  "test_cases": {},
  "stats": {
    "total": 0,
    "written": 0,
    "skipped": 0,
    "failed": 0
  },
  "iterations": []
}
EOF
}

create_test_results_file() {
    cat << 'EOF'
{
  "last_run": null,
  "runs": [],
  "summary": {
    "total_runs": 0,
    "last_passed": 0,
    "last_failed": 0,
    "last_skipped": 0
  },
  "failing_tests": [],
  "flaky_tests": []
}
EOF
}
