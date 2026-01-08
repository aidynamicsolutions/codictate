# /// script
# dependencies = [
#   "fastapi>=0.115.0",
#   "uvicorn>=0.32.0",
#   "mlx-lm>=0.22.0",
#   "pytest>=8.0.0",
#   "httpx>=0.27.0",
# ]
# ///
"""
Test runner for MLX sidecar tests.

Usage:
    uv run python-backend/run_tests.py
"""
import subprocess
import sys

if __name__ == "__main__":
    # Run pytest on the tests directory
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "tests/", "-v"],
        cwd="python-backend" if not __file__.endswith("run_tests.py") else ".",
    )
    sys.exit(result.returncode)
