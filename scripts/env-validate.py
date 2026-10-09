#!/usr/bin/env python3
"""
env-validate CLI — 多语言自动检测入口

用法:
  python scripts/env-validate.py [python|node|bash|go|rust|auto] [--env-file path] [--report path] [--sarif path]

auto 模式（默认）：检测本机可用的运行时，选择第一个可用的执行验证。
选项:
  --env-file <path>   指定 .env 文件，验证前导入其中的变量（不覆盖已存在的）
  --report <path>     生成 JSON 报告到指定路径
  --sarif <path>      生成 SARIF 报告到指定路径
"""

import os
import sys
import shutil
import subprocess

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(SCRIPT_DIR)



def find_validator(language):
    """返回 (cmd_list, display_name) 或 None。"""
    examples = os.path.join(ROOT_DIR, "examples")

    if language == "python":
        py = sys.executable or "python3"
        return [py, os.path.join(examples, "validate.py")], "Python"

    if language == "node":
        node = shutil.which("node")
        if not node:
            return None
        return [node, os.path.join(examples, "validate.js")], "Node.js"

    if language == "bash":
        bash = shutil.which("bash") or shutil.which("sh")
        if not bash:
            return None
        return [bash, os.path.join(examples, "validate.sh")], "Bash"

    if language == "go":
        go = shutil.which("go")
        if not go:
            return None
        return [go, "run", os.path.join(examples, "validate.go")], "Go"

    if language == "rust":
        rustc = shutil.which("rustc")
        if not rustc:
            return None
        bin_path = os.path.join(ROOT_DIR, "target", "env-validate-rust")
        # 尝试已编译的二进制，否则用 rustc 即时编译
        if os.path.isfile(bin_path):
            return [bin_path], "Rust"
        return [rustc, os.path.join(examples, "validate.rs"), "-o", bin_path, "&&", bin_path], "Rust"

    return None


def detect_available():
    """检测本机可用的运行时，返回语言列表。"""
    available = ["python"]  # 保证有
    if shutil.which("node"):
        available.append("node")
    if shutil.which("bash") or shutil.which("sh"):
        available.append("bash")
    if shutil.which("go"):
        available.append("go")
    if shutil.which("rustc"):
        available.append("rust")
    return available


def parse_args():
    """解析 CLI 参数，返回 (mode, env_file, report_path, sarif_path)。"""
    args = sys.argv[1:]
    mode = "auto"
    env_file = ""
    report_path = ""
    sarif_path = ""

    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ("help", "--help", "-h"):
            print("env-validate CLI")
            print("Usage: python scripts/env-validate.py [python|node|bash|go|rust|auto] [options]")
            print()
            print("Modes:")
            print("  auto    自动检测可用运行时，依次执行（默认）")
            print("  python  运行 Python 验证器")
            print("  node    运行 Node.js 验证器")
            print("  bash    运行 Bash 验证器")
            print("  go      运行 Go 验证器")
            print("  rust    运行 Rust 验证器")
            print()
            print("Options:")
            print("  --env-file <path>   指定 .env 文件，验证前导入变量")
            print("  --report <path>     生成 JSON 报告")
            print("  --sarif <path>      生成 SARIF 报告")
            sys.exit(0)
        elif arg in ("python", "node", "bash", "go", "rust", "auto"):
            mode = arg
        elif arg == "--env-file":
            i += 1
            env_file = args[i] if i < len(args) else ""
        elif arg == "--report":
            i += 1
            report_path = args[i] if i < len(args) else ""
        elif arg == "--sarif":
            i += 1
            sarif_path = args[i] if i < len(args) else ""
        else:
            print(f"  ✗ Unknown argument: {arg}")
            sys.exit(2)
        i += 1

    return mode, env_file, report_path, sarif_path


def main():
    mode, env_file, report_path, sarif_path = parse_args()

    # 设置环境变量传递给子验证器
    if env_file:
        os.environ["ENV_VALIDATE_ENV_FILE"] = env_file
    if report_path:
        os.environ["ENV_VALIDATE_REPORT"] = report_path
    if sarif_path:
        os.environ["ENV_VALIDATE_SARIF"] = sarif_path

    if mode == "auto":
        available = detect_available()
        print(f"Available runtimes: {', '.join(available)}\n")

        exit_code = 0
        for lang in available:
            cmd, name = find_validator(lang)
            if not cmd:
                print(f"  ⏭  {name}: not available, skipping")
                continue

            print(f"--- Running {name} validator ---")
            # 对 rust 需要 shell 执行（因为含 &&）
            if "&&" in cmd:
                parts = cmd.split("&&")
                proc1 = subprocess.run(parts[0], capture_output=True, text=True, errors="replace")
                if proc1.returncode != 0:
                    print(f"  compile failed: {proc1.stderr}")
                    exit_code = 1
                    continue
                proc2 = subprocess.run(parts[1], capture_output=True, text=True, errors="replace")
                print(proc2.stdout)
                if proc2.returncode != 0:
                    exit_code = 1
            else:
                proc = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
                print(proc.stdout)
                if proc.stderr:
                    print(proc.stderr, file=sys.stderr)
                if proc.returncode != 0:
                    exit_code = 1
            print()

        sys.exit(exit_code)
    else:
        cmd, name = find_validator(mode)
        if not cmd:
            print(f"  ✗ {mode} runtime not available")
            sys.exit(2)

        print(f"--- Running {name} validator ---")
        if "&&" in cmd:
            parts = cmd.split("&&")
            subprocess.run(parts[0], text=True, errors="replace")
            proc = subprocess.run(parts[1], capture_output=True, text=True, errors="replace")
        else:
            proc = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
        print(proc.stdout)
        if proc.stderr:
            print(proc.stderr, file=sys.stderr)
        sys.exit(proc.returncode)


if __name__ == "__main__":
    main()
