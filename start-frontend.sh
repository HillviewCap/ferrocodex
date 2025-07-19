#!/bin/bash

echo "🚀 Starting Ferrocodex Frontend Server"
echo "====================================="

# Navigate to the desktop app directory
cd /home/jenkins/ferrocodex/apps/desktop

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo "❌ Error: package.json not found in current directory"
    echo "Current directory: $(pwd)"
    exit 1
fi

# Kill any existing processes on port 1420
echo "🔄 Checking for existing processes on port 1420..."
lsof -ti:1420 | xargs kill -9 2>/dev/null || echo "No existing processes on port 1420"

# Start the Vite development server
echo "🌟 Starting Vite development server..."
npm run dev

echo "🎯 Server should be running at http://localhost:1420/"