# Env Manager Lite - 多语言验证器
# 支持 Python / Node.js / Bash / Go / Rust

FROM alpine:3.20

# 安装所有运行时
RUN apk add --no-cache python3 nodejs bash go rust cargo

WORKDIR /app
COPY examples/ /app/examples/
COPY .env-required /app/.env-required
COPY entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/entrypoint.sh /app/examples/validate.sh

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["python"]
