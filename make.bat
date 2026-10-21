@echo off
setlocal

set TARGET=%1
if "%TARGET%"=="" set TARGET=help

if "%DOCS_DIR%"=="" set DOCS_DIR=docs

if /I "%TARGET%"=="help" goto help
if /I "%TARGET%"=="all" goto help
if /I "%TARGET%"=="install_base" goto install_base
if /I "%TARGET%"=="install_deps" goto install_deps
if /I "%TARGET%"=="build_docs" goto build_docs
if /I "%TARGET%"=="docs" goto docs
if /I "%TARGET%"=="build" goto build
if /I "%TARGET%"=="lint" goto lint
if /I "%TARGET%"=="test" goto test
if /I "%TARGET%"=="build_docker" goto build_docker
if /I "%TARGET%"=="run_docker" goto run_docker

echo Unknown target: %TARGET%
goto help

:help
echo Available commands:
echo   install_base   - Install language toolchains (Rust)
echo   install_deps   - Install project dependencies ^& WASM targets
echo   build_docs     - Build the API documentation
echo   docs           - Build and prepare documentation in %DOCS_DIR%
echo   build          - Build the cdd-engine library
echo   lint           - Run cargo fmt and clippy with -D warnings
echo   test           - Run the test suite
echo   build_docker   - Build Alpine and Debian Docker images
echo   run_docker     - Run verification inside Docker containers
goto end

:install_base
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Rust/Cargo is not installed. Please install from https://rustup.rs/
    exit /b 1
)
goto end

:install_deps
call :install_base
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%
cargo fetch
call scripts/fetch_wasm.bat
goto end

:build_docs
echo Generating Rust documentation into %DOCS_DIR%...
cargo doc --no-deps --target-dir %DOCS_DIR%_tmp
if not exist "%DOCS_DIR%" mkdir "%DOCS_DIR%"
xcopy /E /I /Y "%DOCS_DIR%_tmp/doc/*" "%DOCS_DIR%/"
rmdir /S /Q "%DOCS_DIR%_tmp"
goto end

:docs
call :build_docs
echo ^<meta http-equiv="refresh" content="0; url=cdd_engine/index.html"^> > "%DOCS_DIR%/index.html"
echo Documentation generated in %DOCS_DIR%/cdd_engine/index.html
goto end

:build
echo Building cdd-engine...
cargo build --release
goto end

:lint
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
goto end

:test
echo Running unit tests...
cargo test --all-targets
goto end

:build_docker
echo Building Docker images...
docker build -f alpine.Dockerfile -t cdd-engine:alpine .
docker build -f debian.Dockerfile -t cdd-engine:debian .
goto end

:run_docker
call :build_docker
echo Running cdd-engine verification in Alpine container...
docker run --rm cdd-engine:alpine
docker rmi cdd-engine:alpine cdd-engine:debian
goto end

:end
endlocal
