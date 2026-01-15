# Benchmark automation script for Windows

param(
    [Parameter(Position=0)]
    [ValidateSet('all', 'mixer', 'resampling', 'dsp', 'memory', 'flamegraph', 'compare', 'save')]
    [string]$Benchmark = 'all',

    [Parameter(Position=1)]
    [string]$Baseline = 'main'
)

$ErrorActionPreference = 'Stop'

Write-Host "🚀 Troubadour Benchmark Suite" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

# Check if flamegraph is installed
$flamegraphInstalled = $false
try {
    $null = cargo flamegraph --version 2>$null
    $flamegraphInstalled = $true
} catch {
    Write-Host "⚠️  cargo-flamegraph not found. Install with:" -ForegroundColor Yellow
    Write-Host "  cargo install flamegraph" -ForegroundColor Yellow
    Write-Host ""
}

switch ($Benchmark) {
    'mixer' {
        Write-Host "▶️  Running mixer benchmarks..." -ForegroundColor Green
        cargo bench --bench mixer_benchmark
    }
    'resampling' {
        Write-Host "▶️  Running resampling benchmarks..." -ForegroundColor Green
        cargo bench --bench resampling_benchmark
    }
    'dsp' {
        Write-Host "▶️  Running DSP benchmarks..." -ForegroundColor Green
        cargo bench --bench dsp_benchmark
    }
    'memory' {
        Write-Host "▶️  Running memory benchmarks..." -ForegroundColor Green
        cargo bench --bench memory_benchmark
    }
    'flamegraph' {
        if (-not $flamegraphInstalled) {
            Write-Host "❌ flamegraph not installed. Aborting." -ForegroundColor Red
            exit 1
        }
        Write-Host "🔥 Generating flamegraphs..." -ForegroundColor Green
        cargo flamegraph --bench mixer_benchmark --output mixer-flamegraph.svg
        cargo flamegraph --bench resampling_benchmark --output resampling-flamegraph.svg
        Write-Host "✅ Flamegraphs generated:" -ForegroundColor Green
        Write-Host "  - mixer-flamegraph.svg"
        Write-Host "  - resampling-flamegraph.svg"
    }
    'compare' {
        Write-Host "📊 Comparing against baseline: $Baseline" -ForegroundColor Green
        cargo bench --all -- --baseline $Baseline
    }
    'save' {
        Write-Host "💾 Saving baseline: $Baseline" -ForegroundColor Green
        cargo bench --all -- --save-baseline $Baseline
    }
    'all' {
        Write-Host "▶️  Running all benchmarks..." -ForegroundColor Green
        cargo bench --all
    }
}

Write-Host ""
Write-Host "✅ Benchmarks completed!" -ForegroundColor Green
Write-Host ""
Write-Host "View results:"
Write-Host "  Start-Process target\criterion\report\index.html"
