#!/usr/bin/env python3
"""
Animation Asset Manager - 开发环境检查
用法: python scripts/check.py
"""

import shutil
import subprocess
import os
import sys
from pathlib import Path

if sys.platform == "win32":
    import ctypes
    ctypes.windll.kernel32.SetConsoleOutputCP(65001)


def check_tool(name, cmd, install_hint=""):
    """检查工具是否安装"""
    path = shutil.which(cmd[0])
    if path:
        try:
            result = subprocess.run(
                cmd, capture_output=True, text=True, shell=True, timeout=10
            )
            version = result.stdout.strip().splitlines()[0] if result.stdout.strip() else "未知版本"
            print(f"  [OK] {name}: {version}")
            return True
        except Exception:
            print(f"  [OK] {name}: 已安装")
            return True
    else:
        print(f"  [X]  {name}: 未安装")
        if install_hint:
            print(f"       {install_hint}")
        return False


def main():
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    os.chdir(project_root)

    print("=" * 50)
    print(" 开发环境检查")
    print("=" * 50)
    print()

    check_tool("Node.js", ["node", "--version"], "请安装 LTS 版本: https://nodejs.org/")
    check_tool("npm", ["npm", "--version"])
    check_tool("Rust", ["rustc", "--version"], "请运行: https://tauri.app/start/prerequisites/")
    check_tool("Cargo", ["cargo", "--version"])

    print()
    print("[项目依赖]")
    if (project_root / "node_modules" / ".package-lock.json").exists():
        print("  [OK] node_modules 已安装")
    else:
        print("  [!]  node_modules 未安装，请运行 npm install")

    print()
    print("=" * 50)
    try:
        input("按 Enter 键退出...")
    except EOFError:
        pass


if __name__ == "__main__":
    main()
