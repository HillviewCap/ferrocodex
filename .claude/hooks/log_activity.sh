#!/bin/bash

# This script logs the activity of Claude tools to a file.
# It's designed to be used as a PostToolUse hook.

# Define the log file in the project directory
log_file="$CLAUDE_PROJECT_DIR/subagent_activity.log"

# Read the JSON input from stdin
input_json=$(cat)

# Safely parse JSON using jq (a common, robust tool for shell JSON processing)
tool_name=$(echo "$input_json" | jq -r '.tool_name')

# Extract relevant details based on the tool used
details=""
case "$tool_name" in
  "Write" | "Read" | "Edit")
    details=$(echo "$input_json" | jq -r '.tool_input.file_path')
    ;;
  "LS")
    details=$(echo "$input_json" | jq -r '.tool_input.path')
    ;;
  "Bash")
    # For Bash, let's log the first 60 chars of the command
    command=$(echo "$input_json" | jq -r '.tool_input.command')
    details=$(echo "${command:0:60}...")
    ;;
  "Task")
    details=$(echo "$input_json" | jq -r '.tool_input.task')
    ;;
  *)
    # Generic fallback for other tools
    details="-"
    ;;
esac

# Get the current UTC timestamp
timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Format the log message
log_message="[$timestamp] TOOL: $tool_name | DETAILS: $details"

# Append the message to the log file
echo "$log_message" >> "$log_file"

# Exit with 0 to indicate success and allow Claude to continue
exit 0