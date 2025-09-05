param(
  [string]$DatabaseUrl = "postgres://postgres:dev@localhost:5433/gm"
)

$ErrorActionPreference = "Stop"

Write-Host "Preparing SQLx offline metadata using $DatabaseUrl" -ForegroundColor Cyan

$env:DATABASE_URL = $DatabaseUrl

# Ensure DB exists and is at the latest schema
sqlx database create
sqlx migrate run

# Generate .sqlx offline metadata for all targets (bin + tests)
cargo sqlx prepare -- --all-targets

Write-Host ".sqlx prepared. Test an offline build with: (set SQLX_OFFLINE=true) cargo build" -ForegroundColor Green
