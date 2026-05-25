function Get-AiMemoryCwd {
    param([string] $Payload)
    if (-not $Payload) { return $null }
    $match = [regex]::Match($Payload, '"cwd"\s*:\s*"([^"]*)"')
    if ($match.Success) { return $match.Groups[1].Value }
    return $null
}

function Get-AiMemoryMarkerToml {
    param([string] $Cwd)
    if (-not $Cwd) { return $null }
    $dir = $Cwd
    while ($dir -and (Test-Path $dir)) {
        $candidate = Join-Path $dir ".ai-memory.toml"
        if (Test-Path $candidate -PathType Leaf) { return $candidate }
        if ($env:HOME -and $dir -eq $env:HOME) { return $null }
        if ($env:USERPROFILE -and $dir -eq $env:USERPROFILE) { return $null }
        $parent = Split-Path $dir -Parent
        if (-not $parent -or $parent -eq $dir) { return $null }
        $dir = $parent
    }
    return $null
}

function Get-AiMemoryTomlKey {
    param([string] $File, [string] $Key)
    if (-not (Test-Path $File -PathType Leaf)) { return $null }
    foreach ($line in Get-Content $File) {
        $m = [regex]::Match($line, "^\s*$Key\s*=\s*`"([^`"]*)`"")
        if ($m.Success) { return $m.Groups[1].Value }
    }
    return $null
}

function Get-AiMemoryMarkerQuery {
    param([string] $Cwd)
    $marker = Get-AiMemoryMarkerToml -Cwd $Cwd
    if (-not $marker) { return "" }
    $qs = ""
    $ws = Get-AiMemoryTomlKey -File $marker -Key "workspace"
    if ($ws) { $qs += "&workspace=$([uri]::EscapeDataString($ws))" }
    $proj = Get-AiMemoryTomlKey -File $marker -Key "project"
    if ($proj) { $qs += "&project=$([uri]::EscapeDataString($proj))" }
    return $qs
}

function Invoke-AiMemoryHook {
    param(
        [Parameter(Mandatory = $true)] [string] $Event,
        [Parameter(Mandatory = $true)] [string] $Agent,
        [switch] $FetchHandoff
    )

    $Server = if ($env:AI_MEMORY_HOOK_URL) { $env:AI_MEMORY_HOOK_URL } else { "http://127.0.0.1:49374" }
    $Payload = [Console]::In.ReadToEnd()
    $Cwd = Get-AiMemoryCwd -Payload $Payload
    $QS = Get-AiMemoryMarkerQuery -Cwd $Cwd
    $Headers = @{}

    if ($env:AI_MEMORY_AUTH_TOKEN) {
        $Headers["Authorization"] = "Bearer $env:AI_MEMORY_AUTH_TOKEN"
    }

    try {
        Invoke-WebRequest `
            -UseBasicParsing `
            -TimeoutSec 1 `
            -Method Post `
            -Uri "$Server/hook?event=$Event&agent=$Agent$QS" `
            -Headers $Headers `
            -ContentType "application/json" `
            -Body $Payload | Out-Null
    } catch {
    }

    if ($FetchHandoff) {
        try {
            $Response = Invoke-WebRequest `
                -UseBasicParsing `
                -TimeoutSec 1 `
                -Uri "$Server/handoff?agent=$Agent$QS" `
                -Headers $Headers
            if ($null -ne $Response -and $null -ne $Response.Content) {
                [Console]::Out.Write($Response.Content)
            }
        } catch {
        }
    }
}
