#!/bin/bash

# Enhanced error handling and diagnostics
set -euo pipefail
trap 'error_handler $? $LINENO $BASH_COMMAND' ERR
trap 'cleanup_handler' EXIT

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="${SCRIPT_DIR}/setup.log"
STATE_DIR="${SCRIPT_DIR}/.setup_state"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# Create state tracking directory
mkdir -p "$STATE_DIR"

# Enhanced logging function
log() {
    local level=$1
    shift
    echo "[$TIMESTAMP][$level] $*" | tee -a "$LOG_FILE"
}

# Error handler with comprehensive diagnostics
error_handler() {
    local exit_code=$1
    local line_no=$2
    local command="$3"
    
    log "ERROR" "Script failed with exit code $exit_code at line $line_no"
    log "ERROR" "Failed command: $command"
    log "ERROR" "Current working directory: $(pwd)"
    log "ERROR" "Available disk space: $(df -h . | tail -1)"
    log "ERROR" "Available memory: $(free -h | grep Mem)"
    
    # Capture system state for debugging
    if command -v opam &> /dev/null; then
        log "ERROR" "Opam switches: $(opam switch list 2>/dev/null || echo 'Opam not initialized')"
    fi
    
    if command -v rustc &> /dev/null; then
        log "ERROR" "Rust version: $(rustc --version 2>/dev/null || echo 'Rust not available')"
    fi
    
    exit $exit_code
}

# Cleanup handler
cleanup_handler() {
    log "INFO" "Setup script completed or interrupted"
}

# Check if step was already completed
is_step_completed() {
    local step_name=$1
    [ -f "$STATE_DIR/${step_name}.done" ]
}

# Mark step as completed
mark_step_completed() {
    local step_name=$1
    touch "$STATE_DIR/${step_name}.done"
    log "INFO" "Marked step '$step_name' as completed"
}

# Skip step if already completed
run_step() {
    local step_name=$1
    local step_description=$2
    shift 2
    
    if is_step_completed "$step_name"; then
        log "INFO" "Skipping $step_description (already completed)"
        return 0
    fi
    
    log "INFO" "Starting: $step_description"
    "$@"
    mark_step_completed "$step_name"
    log "INFO" "Completed: $step_description"
}

# Enhanced system checks
check_system_requirements() {
    log "INFO" "Checking system requirements..."
    
    # Check available disk space (require at least 2GB)
    local available_space=$(df . | tail -1 | awk '{print $4}')
    if [ "$available_space" -lt 2097152 ]; then
        log "ERROR" "Insufficient disk space. Available: ${available_space}KB, Required: 2097152KB"
        exit 1
    fi
    
    # Check available memory (require at least 1GB)
    local available_memory=$(free | grep Mem | awk '{print $7}')
    if [ "$available_memory" -lt 1048576 ]; then
        log "WARNING" "Low available memory: ${available_memory}KB"
    fi
    
    log "INFO" "System requirements check passed"
}

# Idempotent package installation
install_system_packages() {
    local packages="build-essential curl git opam"
    
    # Check if we're running as root
    if [ "$(id -u)" -eq 0 ]; then
        APT_PREFIX=""
    else
        APT_PREFIX="sudo"
    fi
    
    # Check if packages are already installed
    local missing_packages=""
    for package in $packages; do
        if ! dpkg -l | grep -q "^ii  $package "; then
            missing_packages="$missing_packages $package"
        fi
    done
    
    if [ -n "$missing_packages" ]; then
        log "INFO" "Installing missing packages:$missing_packages"
        $APT_PREFIX apt-get update || {
            log "ERROR" "Failed to update package lists"
            exit 1
        }
        $APT_PREFIX apt-get install -y $missing_packages || {
            log "ERROR" "Failed to install packages:$missing_packages"
            exit 1
        }
    else
        log "INFO" "All required packages already installed"
    fi
}

# Idempotent Opam initialization
initialize_opam() {
    if [ -d "$HOME/.opam" ] && opam switch list &> /dev/null; then
        log "INFO" "Opam already initialized"
        eval $(opam env) 2>/dev/null || true
        return 0
    fi
    
    log "INFO" "Initializing Opam..."
    opam init --bare --disable-sandboxing --no-setup --yes || {
        log "ERROR" "Failed to initialize Opam"
        exit 1
    }
    
    eval $(opam env) || {
        log "ERROR" "Failed to set up Opam environment"
        exit 1
    }
}

# Idempotent Opam switch creation
create_ocaml_switch() {
    local switch_version="5.2.1"
    
    if opam switch list | grep -q "$switch_version"; then
        log "INFO" "OCaml switch $switch_version already exists"
        opam switch set "$switch_version" || {
            log "ERROR" "Failed to set OCaml switch to $switch_version"
            exit 1
        }
    else
        log "INFO" "Creating OCaml switch $switch_version..."
        opam switch create "$switch_version" || {
            log "ERROR" "Failed to create OCaml switch $switch_version"
            exit 1
        }
    fi
    
    eval $(opam env) || {
        log "ERROR" "Failed to set up Opam environment after switch creation"
        exit 1
    }
}

# Idempotent Rust installation
install_rust() {
    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        log "INFO" "Rust already installed: $(rustc --version)"
        export PATH="$HOME/.cargo/bin:$PATH"
        return 0
    fi
    
    log "INFO" "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || {
        log "ERROR" "Failed to install Rust"
        exit 1
    }
    
    # Source environment
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    
    log "INFO" "Rust installed successfully: $(rustc --version)"
}

# Idempotent Rust components installation
install_rust_components() {
    local components="rustfmt clippy"
    
    for component in $components; do
        if rustup component list | grep -q "^$component-.*installed"; then
            log "INFO" "Rust component $component already installed"
        else
            log "INFO" "Installing Rust component: $component"
            rustup component add "$component" || {
                log "ERROR" "Failed to install Rust component: $component"
                exit 1
            }
        fi
    done
}

# Idempotent dependency fetching and building
fetch_and_build() {
    if [ -f "target/debug/deps" ] && [ -d "target/debug/deps" ]; then
        log "INFO" "Cargo dependencies already fetched"
    else
        log "INFO" "Fetching cargo dependencies..."
        cargo fetch || {
            log "ERROR" "Failed to fetch cargo dependencies"
            exit 1
        }
    fi
    
    if [ -f "target/debug/ocaml-lwt-interop" ] || [ -f "target/debug/libocaml_lwt_interop.a" ]; then
        log "INFO" "Project already built"
    else
        log "INFO" "Building project..."
        cargo build || {
            log "ERROR" "Failed to build project"
            exit 1
        }
    fi
}

# Idempotent Opam dependencies installation
install_opam_dependencies() {
    # Check if opam dependencies are already installed
    if opam list --installed | grep -q "lwt\|ocaml-rs-smartptr"; then
        log "INFO" "Opam dependencies appear to be already installed"
    else
        log "INFO" "Installing Opam dependencies..."
        opam install . --deps-only --with-test --no-depexts --yes || {
            log "ERROR" "Failed to install Opam dependencies"
            exit 1
        }
    fi
}

# Main execution flow
main() {
    log "INFO" "Starting OCaml/Rust development environment setup for ocaml-lwt-interop"
    log "INFO" "Script version: Enhanced with idempotency and diagnostics"
    
    check_system_requirements
    
    run_step "system_packages" "Installing system packages" install_system_packages
    run_step "opam_init" "Initializing Opam" initialize_opam
    run_step "ocaml_switch" "Creating OCaml switch" create_ocaml_switch
    run_step "rust_install" "Installing Rust" install_rust
    run_step "rust_components" "Installing Rust components" install_rust_components
    run_step "fetch_build" "Fetching dependencies and building" fetch_and_build
    run_step "opam_deps" "Installing Opam dependencies" install_opam_dependencies
    
    log "INFO" "Setup complete! Environment is ready for ocaml-lwt-interop development"
    log "INFO" "To clean state and force re-run: rm -rf $STATE_DIR"
}

# Execute main function
main "$@"
