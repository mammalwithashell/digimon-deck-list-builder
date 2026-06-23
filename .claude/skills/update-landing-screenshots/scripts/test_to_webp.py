import subprocess, sys, pathlib
from PIL import Image

HERE = pathlib.Path(__file__).parent


def test_converts_png_to_webp_at_target_width(tmp_path):
    src = tmp_path / "in.png"
    Image.new("RGB", (1200, 720), (10, 20, 16)).save(src)
    out = tmp_path / "out.webp"
    subprocess.run(
        [sys.executable, str(HERE / "to_webp.py"), str(src), str(out), "--width", "600"],
        check=True,
    )
    assert out.exists()
    im = Image.open(out)
    assert im.format == "WEBP"
    assert im.width == 600
    assert im.height == 360  # aspect preserved
