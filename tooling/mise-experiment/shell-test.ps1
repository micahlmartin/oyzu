$ErrorActionPreference = 'Stop'
$env:OYZU_PROJECT = 'before'
$env:PATH = $env:OYZU_TEST_EXTRA + ';' + $env:OYZU_TEST_EXTRA + ';' + $env:PATH
$initialPath = $env:PATH
(& $env:OYZU_SPIKE_BINARY activate -s pwsh | Out-String) | Invoke-Expression
Set-Location -LiteralPath $env:OYZU_TEST_A
_mise_hook
if ((node --version) -ne $env:OYZU_EXPECT_A -or $env:OYZU_PROJECT -ne $env:OYZU_EXPECT_ENV_A) { throw 'entry failed' }
Set-Location -LiteralPath $env:OYZU_TEST_B
_mise_hook
if ((node --version) -ne $env:OYZU_EXPECT_B -or $env:OYZU_PROJECT -ne $env:OYZU_EXPECT_ENV_B) { throw 'switch failed' }
Set-Location -LiteralPath $env:OYZU_TEST_OUTSIDE
_mise_hook
if ($env:OYZU_PROJECT -ne 'before' -or $env:PATH -ne $initialPath) { throw 'restoration failed' }
Set-Location -LiteralPath $env:OYZU_TEST_A
_mise_hook
$env:OYZU_PROJECT = 'user-change'
$env:PATH = $env:OYZU_TEST_EXTRA + ';' + $env:PATH
Set-Location -LiteralPath $env:OYZU_TEST_OUTSIDE
_mise_hook
if ($env:OYZU_PROJECT -ne 'user-change') { throw 'user environment edit lost' }
if ($env:PATH -ne ($env:OYZU_TEST_EXTRA + ';' + $initialPath)) { throw 'user PATH edit lost' }
Write-Output 'shell lifecycle passed'
