#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

export host="127.0.0.1"
export port="8181"

export JWT_ALGORITHM=RS512
export JWT_PUBLIC_CERTIFICATE="$(cat "$SCRIPT_DIR/../app/.env" | grep SESSION_JWT_PUBLIC_CERTIFICATE | sed 's/SESSION_JWT_PUBLIC_CERTIFICATE=//' | sed 's/\"//g')"
export POLICY_INFORMATION_POINT_TOKEN="$(cat "$SCRIPT_DIR/../policy-information-point/.env" | grep AUTH_TOKEN | sed 's/AUTH_TOKEN=//' | sed 's/\"//g')"
export POLICY_INFORMATION_POINT_URL="http://$host:$(cat "$SIDECHAIN/../policy-information-point/.env" | grep REST_PORT | sed 's/REST_PORT=//' | sed 's/\"//g')"
export USE_POLICY_INFORMATION_POINT=true

export BASE_URL="http://$host:$port"
export ENVIRONMENT="local"

export OTEL_HOST="127.0.0.1"
export OTEL_PORT="4317"

export POLICY_PUSH_INTERVAL="2"
export POLICY_PUSH_BATCH_SIZE="10"

run_opa() {
  opa build \
    -b "$SCRIPT_DIR/rego" \
    -o "$SCRIPT_DIR/bundle.tar.gz" \
    --ignore "test*"
  opa run --server \
    --addr ":$port" \
    --bundle "$SCRIPT_DIR/bundle.tar.gz" \
    --disable-telemetry \
    --log-level debug \
    --log-format text \
    --config-file "$SCRIPT_DIR/config.yaml" &
  export opa_pid=$!
}
export -f run_opa

restart() {
  echo "restarting opa..."
  sleep 0.1
  end
  run_opa
}
export -f restart

end() {
  lsof -i ":$port" |
    grep "^opa  " |
    sed 's/opa *//g' |
    sed 's/ .*//g' |
    xargs -n1 kill -9
}
export -f end

run_opa

trap end INT TERM EXIT

cargo watch -l 0.3 -o "$SCRIPT_DIR" | xargs -n1 bash -c restart
