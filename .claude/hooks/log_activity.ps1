# This script logs the activity of Claude tools to a file.
# It's a PowerShell version designed for Windows.

# Get the project directory from the environment variable
$projectDir = $env:CLAUDE_PROJECT_DIR
$logFile = Join-Path -Path $projectDir -ChildPath "subagent_activity.log"

# Read JSON from stdin and convert it
$inputJson = $input | ConvertFrom-Json

$toolName = $inputJson.tool_name

# Extract relevant details based on the tool used
$details = ""
switch ($toolName) {
    "Write" { $details = $inputJson.tool_input.file_path }
    "Read"  { $details = $inputJson.tool_input.file_path }
    "Edit"  { $details = $inputJson.tool_input.file_path }
    "LS"    { $details = $inputJson.tool_input.path }
    "Bash"  {
        $command = $inputJson.tool_input.command
        if ($command.Length -gt 60) {
            $details = $command.Substring(0, 60) + "..."
        } else {
            $details = $command
        }
    }
    "Task"  { $details = $inputJson.tool_input.task }
    default { $details = "-" }
}

# Get current UTC timestamp in ISO 8601 format
$timestamp = (Get-Date).ToUniversalTime().ToString("u")

# Format the log message
$logMessage = "[$timestamp] TOOL: $toolName | DETAILS: $details"

# Append the message to the log file
Add-Content -Path $logFile -Value $logMessage

# Exit with 0 to indicate success
exit 0