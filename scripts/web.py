#!/usr/bin/env python3
"""
Animation Asset Manager - 启动前端 Vite 开发服务器
用法: python scripts/web.py
"""

import subprocess
import sys
import os
from pathlib import Path

if sys.platform == "win32":
    import ctypes
    ctypes.windll.kernel32.SetConsoleOutputCP(65001)


def main():
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    os.chdir(project_root)

    print("=" * 50)
    print(" Animation Asset Manager - 前端开发模式")
    print("=" * 50)
    print()

    if not (project_root / "node_modules" / ".package-lock.json").exists():
        print("[1/2] 未找到 node_modules，正在安装依赖...")
        result = subprocess.run(["npm", "install"], shell=True)
        if result.returncode != 0:
            print("[错误] npm install 失败")
            sys.exit(1)
    else:
        print("[1/2] node_modules 已存在")

    print("[2/2] 启动 Vite 开发服务器...")
    print("      按 Ctrl+C 停止")
    print()

    try:
        subprocess.run(["npm", "run", "dev"], shell=True)
    except KeyboardInterrupt:
        print("\n已停止")


if __name__ == "__main__":
    main()
