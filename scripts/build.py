#!/usr/bin/env python3
"""
Animation Asset Manager - 构建 Windows 发布版
用法: python scripts/build.py
"""

import subprocess
import sys
import os
from pathlib import Path

if sys.platform == "win32":
    import ctypes
    ctypes.windll.kernel32.SetConsoleOutputCP(65001)


def run_cmd(cmd, step_name):
    """运行命令并检查返回值"""
    print(f"[{step_name}] 运行: {' '.join(cmd)}")
    result = subprocess.run(cmd, shell=True)
    if result.returncode != 0:
        print(f"[错误] {step_name} 失败")
        sys.exit(1)
    print(f"[{step_name}] 完成")
    print()


def main():
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    os.chdir(project_root)

    print("=" * 50)
    print(" Animation Asset Manager - Windows 构建")
    print("=" * 50)
    print()

    # 安装依赖
    if (project_root / "node_modules" / ".package-lock.json").exists():
        run_cmd(["npm", "ci"], "安装精确依赖 (npm ci)")
    else:
        run_cmd(["npm", "install"], "安装依赖 (npm install)")

    # 构建前端
    run_cmd(["npm", "run", "build"], "构建前端")

    # 构建 Tauri
    run_cmd(["npx", "tauri", "build"], "构建 Tauri Windows 安装包")

    # 打开输出目录
    bundle_dir = project_root / "src-tauri" / "target" / "release" / "bundle"
    if bundle_dir.exists():
        print("=" * 50)
        print(" [成功] 构建完成！")
        print(f" 安装包位置: {bundle_dir}")
        print("=" * 50)
        os.startfile(bundle_dir)
    else:
        print("[警告] 未找到安装包目录")

    try:
        input("\n按 Enter 键退出...")
    except EOFError:
        pass


if __name__ == "__main__":
    main()
