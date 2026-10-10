<#
.SYNOPSIS
  Records every system value Raftaar can change, and compares two recordings.

.EXAMPLE
  .\state-diff.ps1 -Save before.json      # before optimizing
  .\state-diff.ps1 -Compare before.json   # after Revert All: should print "0 differences"

  Keep the value list in sync with engine/src/catalog.rs and engine/src/restore.rs.
#>
param(
    [string]$Save,
    [string]$Compare
)

$ErrorActionPreference = 'Stop'
$mm = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile'
$gcs = 'HKCU:\System\GameConfigStore'
$cdm = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager'
$cv = 'HKCU:\Software\Microsoft\Windows\CurrentVersion'

$values = @(
    # FPS & input lag
    @('HKCU:\Software\Microsoft\GameBar', 'AutoGameModeEnabled'),
    @('HKCU:\Software\Microsoft\GameBar', 'AllowAutoGameMode'),
    @($gcs, 'GameDVR_Enabled'),
    @("$cv\GameDVR", 'AppCaptureEnabled'),
    @('HKCU:\Control Panel\Mouse', 'MouseSpeed'),
    @('HKCU:\Control Panel\Mouse', 'MouseThreshold1'),
    @('HKCU:\Control Panel\Mouse', 'MouseThreshold2'),
    @($mm, 'SystemResponsiveness'),
    @("$mm\Tasks\Games", 'GPU Priority'),
    @("$mm\Tasks\Games", 'Priority'),
    @("$mm\Tasks\Games", 'Scheduling Category'),
    @('HKLM:\SYSTEM\CurrentControlSet\Control\PriorityControl', 'Win32PrioritySeparation'),
    @('HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\kernel', 'GlobalTimerResolutionRequests'),
    @('HKLM:\SYSTEM\CurrentControlSet\Control\GraphicsDrivers', 'HwSchMode'),
    @($gcs, 'GameDVR_FSEBehaviorMode'),
    @($gcs, 'GameDVR_FSEBehavior'),
    @($gcs, 'GameDVR_HonorUserFSEBehaviorMode'),
    @($gcs, 'GameDVR_DXGIHonorFSEWindowsCompatible'),
    @('HKLM:\SOFTWARE\Microsoft\Windows\Dwm', 'OverlayTestMode'),
    # Power
    @('HKLM:\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling', 'PowerThrottlingOff'),
    # Background
    @("$cv\BackgroundAccessApplications", 'GlobalUserDisabled'),
    @("$cv\Explorer\Serialize", 'StartupDelayInMSec'),
    @('HKLM:\SOFTWARE\Microsoft\Windows\Windows Error Reporting', 'Disabled'),
    @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppCompat', 'DisablePCA'),
    @('HKLM:\SOFTWARE\Policies\Microsoft\Edge', 'StartupBoostEnabled'),
    @('HKLM:\SOFTWARE\Policies\Microsoft\Edge', 'BackgroundModeEnabled'),
    @('HKLM:\SOFTWARE\Policies\Google\Chrome', 'BackgroundModeEnabled'),
    # Ads & tracking
    @("$cv\AdvertisingInfo", 'Enabled'),
    @($cdm, 'SystemPaneSuggestionsEnabled'),
    @($cdm, 'SilentInstalledAppsEnabled'),
    @($cdm, 'SoftLandingEnabled'),
    @($cdm, 'SubscribedContent-338388Enabled'),
    @($cdm, 'SubscribedContent-338389Enabled'),
    @($cdm, 'SubscribedContent-353694Enabled'),
    @($cdm, 'SubscribedContent-353696Enabled'),
    @("$cv\Privacy", 'TailoredExperiencesWithDiagnosticDataEnabled'),
    @("$cv\Search", 'BingSearchEnabled'),
    @("$cv\Search", 'CortanaConsent'),
    @('HKCU:\Software\Policies\Microsoft\Windows\Explorer', 'DisableSearchBoxSuggestions'),
    @('HKLM:\SOFTWARE\Policies\Microsoft\Windows\DataCollection', 'AllowTelemetry'),
    # Visuals
    @("$cv\Explorer\VisualEffects", 'VisualFXSetting'),
    @("$cv\Themes\Personalize", 'EnableTransparency'),
    @('HKCU:\Control Panel\Desktop', 'MenuShowDelay'),
    # Network
    @($mm, 'NetworkThrottlingIndex'),
    # Storage
    @('HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem', 'NtfsDisableLastAccessUpdate'),
    # Restore point helper key
    @('HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore', 'SystemRestorePointCreationFrequency')
)

function Get-State {
    $state = [ordered]@{}
    foreach ($v in $values) {
        $key = "$($v[0])\$($v[1])"
        $item = Get-ItemProperty -Path $v[0] -Name $v[1] -ErrorAction SilentlyContinue
        if ($null -eq $item) {
            $state[$key] = '<missing>'
        } else {
            $state[$key] = "$($item.($v[1]))"
        }
    }
    $active = (powercfg /getactivescheme) -join ' '
    $state['power:active'] = ([regex]'[0-9a-fA-F-]{36}').Match($active).Value
    $plans = powercfg /list | ForEach-Object { ([regex]'[0-9a-fA-F-]{36}').Match($_).Value } | Where-Object { $_ }
    $state['power:plans'] = (@($plans) | Sort-Object) -join ','
    return $state
}

if ($Save) {
    Get-State | ConvertTo-Json | Set-Content -Path $Save -Encoding utf8
    Write-Output "Saved $($values.Count + 2) values to $Save"
    exit 0
}

if ($Compare) {
    $before = Get-Content -Path $Compare -Raw | ConvertFrom-Json
    $now = Get-State
    $diffs = 0
    foreach ($key in $now.Keys) {
        $old = $before.$key
        if ("$old" -ne "$($now[$key])") {
            Write-Output "DIFF $key`n  before: $old`n  now:    $($now[$key])"
            $diffs++
        }
    }
    Write-Output "$diffs differences"
    exit [int]($diffs -gt 0)
}

Get-State | Format-Table -AutoSize
