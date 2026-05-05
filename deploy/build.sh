#!/bin/bash
set -e

echo "Building Agri-IoT System..."

# 构建后端
cargo build --release -p agri-server

# 构建前端 (需要 trunk)
if command -v trunk &> /dev/null; then
    cd agri-frontend
    trunk build --release
    cd ..
    echo "Frontend built successfully"
else
    echo "trunk not found, skipping frontend build"
    echo "Install with: cargo install trunk"
fi

echo "Build complete!"
echo "Binary: target/release/agri-server"
