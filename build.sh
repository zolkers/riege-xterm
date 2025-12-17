#!/bin/bash

# Colors
CYAN="\e[36m"
GREEN="\e[32m"
YELLOW="\e[33m"
RESET="\e[0m"

echo -e "${CYAN}Building Rust terminal library...${RESET}"

# Use cargo from full path (works in WSL)
CARGO="$HOME/.cargo/bin/cargo"

# Check if Cargo.lock has version issues and regenerate if needed
if [ -f "Cargo.lock" ]; then
    if $CARGO build --release 2>&1 | grep -q "lock file version"; then
        echo -e "${YELLOW}Cargo.lock version mismatch, regenerating...${RESET}"
        rm Cargo.lock
    fi
fi

# Build in release mode
$CARGO build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Build successful!${RESET}"

    # Copy to Java natives directory
    NATIVES_DIR="/mnt/c/Users/riege/IdeaProjects/rmc/natives"

    if [ -d "$NATIVES_DIR" ]; then
        echo -e "${CYAN}Copying library to Java natives directory...${RESET}"
        cp target/release/libriege_xterm.so "$NATIVES_DIR/"
        echo -e "${GREEN}✓ Library copied to $NATIVES_DIR${RESET}"
    else
        echo -e "${YELLOW}Warning: Natives directory not found at $NATIVES_DIR${RESET}"
        echo -e "${YELLOW}Library is available at: $(pwd)/target/release/libriege_xterm.so${RESET}"
    fi
else
    echo -e "\e[31m✗ Build failed!${RESET}"
    exit 1
fi
