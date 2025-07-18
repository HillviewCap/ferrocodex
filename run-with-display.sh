#!/bin/bash

# Script to run the application with a virtual display

echo "Setting up virtual display environment..."

# Try to use existing display or create a virtual one
if [ -z "$DISPLAY" ]; then
    echo "No DISPLAY environment variable set. Setting up virtual display..."
    export DISPLAY=:99
fi

# Check if we can use xvfb-run
if command -v xvfb-run &> /dev/null; then
    echo "Using xvfb-run to create virtual display"
    exec xvfb-run -a --server-args="-screen 0 1024x768x24" "$@"
else
    echo "xvfb-run not available. Trying alternative approaches..."
    
    # Try to start a virtual X server manually
    if command -v Xvfb &> /dev/null; then
        echo "Starting Xvfb manually..."
        Xvfb :99 -screen 0 1024x768x24 &
        XVFB_PID=$!
        export DISPLAY=:99
        
        # Wait a moment for X server to start
        sleep 2
        
        # Run the command
        "$@"
        
        # Clean up
        kill $XVFB_PID 2>/dev/null
    else
        echo "Warning: No virtual display available. Attempting to run anyway..."
        echo "You may need to install xvfb: sudo apt-get install xvfb"
        export DISPLAY=:0
        "$@"
    fi
fi