import json
from pathlib import Path
import pytest
import subprocess

_TESTS_DIR = Path(__file__).parent
_DATA_DIR = _TESTS_DIR / "data"
_CHEZMOI_INI_MANAGER = _TESTS_DIR.parent / "bin" / "chezmoi_ini_manager.py"

# Discover tests
_TESTS = [e.stem for e in _DATA_DIR.glob("*.json")]


@pytest.mark.parametrize("name", _TESTS)
def test_example(name: str):
    base_name = _DATA_DIR / name
    with base_name.with_suffix(".json").open(mode="rt") as f:
        options = json.load(f)
    with base_name.with_suffix(".sys.ini").open(mode="rb") as sys_file:

        result = subprocess.run(
            [_CHEZMOI_INI_MANAGER, "-s", base_name.with_suffix(".src.ini")]
            + options["args"],
            stdin=sys_file,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    assert result.returncode == options.get("expected_retcode", 0)
    assert result.stderr.decode("utf-8", errors="strict") == options.get(
        "expected_stderr", ""
    )
    with base_name.with_suffix(".expected.ini").open(mode="rb") as expected_file:
        expected_data = expected_file.read()
        assert expected_data.split(b"\n") == result.stdout.split(b"\n")
