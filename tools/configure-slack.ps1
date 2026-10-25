param(
  [Parameter(Mandatory=$true)][string]$ConfigurationPath,
  [Parameter(Mandatory=$true)][string]$HostTokenFile,
  [Parameter(Mandatory=$true)][ValidateRange(1,65535)][int]$Port
)
$ErrorActionPreference = 'Stop'
# The JSON file contains only workspaceId, botUserId and bindings. Never put tokens in it.
$configuration = Get-Content -LiteralPath $ConfigurationPath -Raw | ConvertFrom-Json
$body = @{
  workspaceId = $configuration.workspaceId
  botUserId = $configuration.botUserId
  bindings = @($configuration.bindings)
}
function Read-LocalSecret([string]$Prompt) {
  $secret = Read-Host -Prompt $Prompt -AsSecureString
  $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secret)
  try { [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer) }
  finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer); $secret.Dispose() }
}
try {
  $body.botToken = Read-LocalSecret 'Slack bot token (xoxb-)'
  $body.signingSecret = Read-LocalSecret 'Slack signing secret'
  $hostBearer = (Get-Content -LiteralPath $HostTokenFile -Raw).Trim()
  Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Port/api/setSlackChannel" `
    -Headers @{ Authorization = "Bearer $hostBearer" } -ContentType 'application/json' `
    -Body ($body | ConvertTo-Json -Depth 8)
}
finally { $body.Clear(); $hostBearer = $null }
