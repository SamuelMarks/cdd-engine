@echo off
setlocal enabledelayedexpansion

if not exist cdd-ctl-wasm-sdk\assets\wasm mkdir cdd-ctl-wasm-sdk\assets\wasm
if not exist target\wasm-cache mkdir target\wasm-cache

echo { > cdd-ctl-wasm-sdk\assets\wasm-support.json

set REPOS=SamuelMarks/cdd-c SamuelMarks/cdd-cpp SamuelMarks/cdd-csharp SamuelMarks/cdd-go SamuelMarks/cdd-java SamuelMarks/cdd-kotlin SamuelMarks/cdd-php SamuelMarks/cdd-python-all SamuelMarks/cdd-ruby SamuelMarks/cdd-rust SamuelMarks/cdd-sh SamuelMarks/cdd-swift SamuelMarks/cdd-ts

set COUNT=0
for %%R in (%REPOS%) do set /a TOTAL+=1

for %%R in (%REPOS%) do (
    set /a COUNT+=1
    for /f "tokens=2 delims=/" %%T in ("%%R") do set TOOL=%%T
    set LANG=!TOOL:cdd-=!
    echo Checking %%R for WASM artifacts (!LANG!)...
    set SUPPORTED=false

    if exist target\wasm-cache\!TOOL!\Cargo.toml (
        pushd target\wasm-cache\!TOOL!
        cargo build --target wasm32-wasip1 --release
        if exist target\wasm32-wasip1elease\!TOOL!.wasm (
            copy /Y target\wasm32-wasip1elease\!TOOL!.wasm ..\..\..\cdd-ctl-wasm-sdk\assets\wasm\!TOOL!.wasm >nul
            set SUPPORTED=true
        )
        popd
    )

    if !COUNT! LSS !TOTAL! (
        echo   "!LANG!": !SUPPORTED!,>> cdd-ctl-wasm-sdk\assets\wasm-support.json
    ) else (
        echo   "!LANG!": !SUPPORTED!>> cdd-ctl-wasm-sdk\assets\wasm-support.json
    )
)

echo } >> cdd-ctl-wasm-sdk\assets\wasm-support.json
echo WASM acquisition complete. Support matrix generated at cdd-ctl-wasm-sdk\assets\wasm-support.json
