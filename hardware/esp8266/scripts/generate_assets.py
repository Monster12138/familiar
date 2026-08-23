Import("env")

from pathlib import Path
import subprocess
import sys

project_dir = Path(env.subst("$PROJECT_DIR"))
repo_root = project_dir.parents[1]
subprocess.run(
    [
        sys.executable,
        str(project_dir / "tools" / "convert_tabby_assets.py"),
        "--source",
        str(repo_root / "sprites" / "tabby-cat"),
        "--output",
        str(project_dir),
    ],
    check=True,
)
