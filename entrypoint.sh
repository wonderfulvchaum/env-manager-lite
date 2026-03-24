#!/bin/sh
MODE=${1:-python}

echo "🚀 Starting Env Validator in $MODE mode..."

case $MODE in
  python)
    python3 examples/validate.py
    ;;
  node)
    node examples/validate.js
    ;;
  bash)
    sh examples/validate.sh
    ;;
  *)
    echo "Unknown mode: $MODE. Use 'python', 'node', or 'bash'."
    exit 1
    ;;
esac
