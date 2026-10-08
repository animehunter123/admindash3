#!/bin/bash

clear
echo "Starting the http://localhost:9999/ DEBUGMODE - Dioxus Web Server (Slow!!!)"
cd ./app* 2>/dev/null 1>/dev/null

echo "Killing any dx processes, and restarting it now..."
pkill -9 dx
dx serve --port 9999 --addr 0.0.0.0 # For final mode, use --release, the wasm bundle will be 2kb.
