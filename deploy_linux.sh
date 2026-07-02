#!/bin/bash
# Aether ACP Agent Wrapper Linux Deployment Script

set -e

echo -e "\e[36mChecking Rust dependencies...\e[0m"
if ! command -v cargo &> /dev/null
then
    echo -e "\e[33mWarning: Cargo/Rust is not detected on the PATH.\e[0m"
    echo -e "\e[33mPlease install Rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\e[0m"
else
    echo -e "\e[32mRust detected. Compiling release binary...\e[0m"
    cargo build --release
    echo -e "\e[32mCompilation complete. Binary: target/release/acp-agent-wrapper\e[0m"
fi

echo -e "\n\e[36mInitializing local isolated workspace directory...\e[0m"
mkdir -p workspace
echo -e "\e[32mWorkspace initialized.\e[0m"

echo -e "\n\e[36mUsage:\e[0m"
echo -e "Run agent wrapper: \e[32m./target/release/acp-agent-wrapper\e[0m"
