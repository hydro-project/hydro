#!/bin/bash
# Kill all zombie DFIR baseline servers and clients

echo "Killing processes on ports 8080-8084..."
for port in 8080 8081 8082 8083 8084; do
    lsof -ti :$port | xargs kill -9 2>/dev/null || true
done

echo "Killing any remaining server/client processes..."
pkill -9 -f "server" 2>/dev/null || true
pkill -9 -f "server_unbounded" 2>/dev/null || true
pkill -9 -f "server_tcp_unbounded" 2>/dev/null || true
pkill -9 -f "server_single_stage" 2>/dev/null || true
pkill -9 -f "server_multistage_admission" 2>/dev/null || true
pkill -9 -f "client_quiet" 2>/dev/null || true
pkill -9 -f "client_rude" 2>/dev/null || true
pkill -9 -f "client_openloop" 2>/dev/null || true

sleep 1
echo "Done"
