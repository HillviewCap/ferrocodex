#!/bin/bash

echo "🚀 Starting Frontend Testing Mode"
echo "================================="

# Start the frontend development server
echo "Starting Vite development server..."
npm run dev &
VITE_PID=$!

# Wait for server to start
sleep 5

echo ""
echo "✅ Frontend server is running at: http://localhost:1420/"
echo ""
echo "🧪 You can now test the UI by:"
echo "   1. Opening http://localhost:1420/ in a browser"
echo "   2. Testing the admin setup workflow"
echo "   3. Testing login functionality"
echo "   4. Testing dashboard features"
echo ""
echo "⚠️  Note: Backend Tauri commands won't work in browser mode"
echo "   But you can test the UI components and flows"
echo ""
echo "Press Ctrl+C to stop the server"

# Wait for user to stop
wait $VITE_PID