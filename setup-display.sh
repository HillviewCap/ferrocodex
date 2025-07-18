#!/bin/bash

echo "🔧 Setting up display environment for GUI testing"
echo "================================================="

# Check if running as root/sudo
if [[ $EUID -eq 0 ]]; then
    echo "Installing xvfb..."
    apt-get update
    apt-get install -y xvfb
    
    echo "✅ xvfb installed successfully"
else
    echo "⚠️  This script needs to be run with sudo to install xvfb"
    echo "Please run: sudo ./setup-display.sh"
    exit 1
fi

# Create startup script
cat > /home/jenkins/ferrocodex/start-with-display.sh << 'EOF'
#!/bin/bash

echo "🚀 Starting application with virtual display"

# Start virtual display
Xvfb :99 -screen 0 1024x768x24 &
XVFB_PID=$!

# Set display environment
export DISPLAY=:99

# Wait for display to initialize
sleep 2

# Start the application
npm run tauri:dev

# Cleanup
kill $XVFB_PID 2>/dev/null
EOF

chmod +x /home/jenkins/ferrocodex/start-with-display.sh

echo "✅ Setup complete! You can now run:"
echo "   ./start-with-display.sh"