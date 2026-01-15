#!/usr/bin/env bash
# Benchmark automation script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Troubadour Benchmark Suite${NC}"
echo "========================================"
echo ""

# Check if flamegraph is installed
if ! command -v cargo-flamegraph &> /dev/null; then
    echo -e "${YELLOW}⚠️  cargo-flamegraph not found. Install with:${NC}"
    echo "  cargo install flamegraph"
    echo ""
fi

# Parse arguments
BENCHMARK=${1:-all}
BASELINE=${2:-main}

case $BENCHMARK in
    mixer)
        echo -e "${GREEN}▶️  Running mixer benchmarks...${NC}"
        cargo bench --bench mixer_benchmark
        ;;
    resampling)
        echo -e "${GREEN}▶️  Running resampling benchmarks...${NC}"
        cargo bench --bench resampling_benchmark
        ;;
    dsp)
        echo -e "${GREEN}▶️  Running DSP benchmarks...${NC}"
        cargo bench --bench dsp_benchmark
        ;;
    memory)
        echo -e "${GREEN}▶️  Running memory benchmarks...${NC}"
        cargo bench --bench memory_benchmark
        ;;
    flamegraph)
        if ! command -v cargo-flamegraph &> /dev/null; then
            echo -e "${RED}❌ flamegraph not installed. Aborting.${NC}"
            exit 1
        fi
        echo -e "${GREEN}🔥 Generating flamegraphs...${NC}"
        cargo flamegraph --bench mixer_benchmark --output mixer-flamegraph.svg
        cargo flamegraph --bench resampling_benchmark --output resampling-flamegraph.svg
        echo -e "${GREEN}✅ Flamegraphs generated:${NC}"
        echo "  - mixer-flamegraph.svg"
        echo "  - resampling-flamegraph.svg"
        ;;
    compare)
        echo -e "${GREEN}📊 Comparing against baseline: ${BASELINE}${NC}"
        cargo bench --all -- --baseline $BASELINE
        ;;
    save)
        echo -e "${GREEN}💾 Saving baseline: ${BASELINE}${NC}"
        cargo bench --all -- --save-baseline $BASELINE
        ;;
    all)
        echo -e "${GREEN}▶️  Running all benchmarks...${NC}"
        cargo bench --all
        ;;
    *)
        echo "Usage: ./scripts/bench.sh [command] [baseline]"
        echo ""
        echo "Commands:"
        echo "  all         Run all benchmarks (default)"
        echo "  mixer       Run mixer engine benchmarks"
        echo "  resampling  Run resampling benchmarks"
        echo "  dsp         Run DSP effect benchmarks"
        echo "  memory      Run memory benchmarks"
        echo "  flamegraph  Generate flamegraphs"
        echo "  compare     Compare against baseline (default: main)"
        echo "  save        Save baseline (default: main)"
        echo ""
        echo "Examples:"
        echo "  ./scripts/bench.sh all"
        echo "  ./scripts/bench.sh mixer"
        echo "  ./scripts/bench.sh compare previous-branch"
        echo "  ./scripts/bench.sh save experimental"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ Benchmarks completed!${NC}"
echo ""
echo "View results:"
echo "  Firefox: firefox target/criterion/report/index.html"
echo "  Chrome:  google-chrome target/criterion/report/index.html"
