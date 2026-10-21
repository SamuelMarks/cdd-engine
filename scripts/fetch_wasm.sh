#!/usr/bin/env bash
set -euo pipefail

mkdir -p cdd-ctl-wasm-sdk/assets/wasm
mkdir -p target/wasm-cache

echo "{" > cdd-ctl-wasm-sdk/assets/wasm-support.json

REPOS=(
    "SamuelMarks/cdd-c"
    "SamuelMarks/cdd-cpp"
    "SamuelMarks/cdd-csharp"
    "SamuelMarks/cdd-go"
    "SamuelMarks/cdd-java"
    "SamuelMarks/cdd-kotlin"
    "SamuelMarks/cdd-php"
    "SamuelMarks/cdd-python-all"
    "SamuelMarks/cdd-ruby"
    "SamuelMarks/cdd-rust"
    "SamuelMarks/cdd-sh"
    "SamuelMarks/cdd-swift"
    "SamuelMarks/cdd-ts"
)

TOTAL=${#REPOS[@]}
COUNT=0

for REPO in "${REPOS[@]}"; do
    COUNT=$((COUNT + 1))
    TOOL=$(basename "$REPO")
    LANG=${TOOL#cdd-}
    echo "Checking $REPO for WASM artifacts ($LANG)..."

    WASM_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep "browser_download_url.*wasm" | cut -d '"' -f 4 | head -n 1 || true)
    SUPPORTED="false"

    if [ -n "$WASM_URL" ]; then
        echo "  -> Downloading release WASM from $WASM_URL"
        curl -sL "$WASM_URL" -o "cdd-ctl-wasm-sdk/assets/wasm/$TOOL.wasm"
        SUPPORTED="true"
    else
        echo "  -> No release WASM found. Attempting local WASM compilation..."
        if [ ! -d "target/wasm-cache/$TOOL" ]; then
            git clone --depth 1 "https://github.com/$REPO.git" "target/wasm-cache/$TOOL" || true
        fi

        if [ -d "target/wasm-cache/$TOOL" ]; then
            pushd "target/wasm-cache/$TOOL" > /dev/null
            if [ -f "Cargo.toml" ]; then
                echo "  -> Rust project detected. Building wasm32-wasip1..."
                if cargo build --target wasm32-wasip1 --release 2>/dev/null; then
                    cp "target/wasm32-wasip1/release/$TOOL.wasm" "../../../cdd-ctl-wasm-sdk/assets/wasm/$TOOL.wasm" 2>/dev/null && SUPPORTED="true" || true
                fi
            elif [ -f "go.mod" ]; then
                echo "  -> Go project detected. Building GOOS=wasip1 GOARCH=wasm..."
                if GOOS=wasip1 GOARCH=wasm go build -o "../../../cdd-ctl-wasm-sdk/assets/wasm/$TOOL.wasm" . 2>/dev/null; then
                    SUPPORTED="true"
                fi
            fi
            popd > /dev/null
        fi
    fi

    if [ $COUNT -eq "$TOTAL" ]; then
        echo "  "$LANG": $SUPPORTED" >> cdd-ctl-wasm-sdk/assets/wasm-support.json
    else
        echo "  "$LANG": $SUPPORTED," >> cdd-ctl-wasm-sdk/assets/wasm-support.json
    fi
done

echo "}" >> cdd-ctl-wasm-sdk/assets/wasm-support.json
echo "WASM acquisition complete. Support matrix generated at cdd-ctl-wasm-sdk/assets/wasm-support.json"
