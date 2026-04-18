#!/usr/bin/env python3
"""
Animation Asset Manager - 启动 Tauri 开发模式
用法: python scripts/dev.py
"""

import subprocess
import sys
import os
from pathlib import Path

if sys.platform == "win32":
    import ctypes
    ctypes.windll.kernel32.SetConsoleOutputCP(65001)


def main():
    # 定位项目根目录
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    os.chdir(project_root)

    print("=" * 50)
    print(" Animation Asset Manager - 开发模式启动")
    print("=" * 50)
    print()

    # 检查 node_modules
    if not (project_root / "node_modules" / ".package-lock.json").exists():
        print("[1/2] 未找到 node_modules，正在安装依赖...")
        result = subprocess.run(["npm", "install"], shell=True)
        if result.returncode != 0:
            print("[错误] npm install 失败")
            sys.exit(1)
    else:
        print("[1/2] node_modules 已存在")

    print("[2/2] 启动 Tauri 开发模式...")
    print("      按 Ctrl+C 停止")
    print()

    try:
        subprocess.run(["npx", "tauri", "dev"], shell=True)
    except KeyboardInterrupt:
        print("\n已停止")


if __name__ == "__main__":
    main()
