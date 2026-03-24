# Stage 1: Build/Prepare (Optional if we just copy scripts, but good for showing skills)
FROM python:3.9-slim as builder
WORKDIR /app
COPY examples/validate.py .

# Stage 2: Runtime (Alpine for minimal size)
FROM alpine:3.18
LABEL maintainer="YourName <your@email.com>"
LABEL description="Lightweight Env Validator Tool"

# Install Node.js and Python runtime in one image to support all scripts
RUN apk add --no-cache python3 nodejs npm bash

WORKDIR /app
COPY examples/ /app/examples/
COPY entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/entrypoint.sh /app/examples/*.sh

# Default command runs the Python validator, but can be overridden
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["python"]
