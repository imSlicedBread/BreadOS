param([ValidateSet('check-env','build','image','smoke','run')][string]$Task = 'image')
$ErrorActionPreference = 'Stop'
cargo xtask $Task
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
