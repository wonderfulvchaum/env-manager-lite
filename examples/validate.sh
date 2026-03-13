#!/bin/bash
echo "Local Environment Validator"
if [ -z "$MY_SECRET_VAR" ]; then
  echo "⚠️ Warning: MY_SECRET_VAR is not set locally."
  echo "💡 Tip: Use 'export MY_SECRET_VAR=your_value' to set it."
else
  echo "✅ Variable is set."
fi
