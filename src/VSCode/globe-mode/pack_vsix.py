"""VSIX（VS Code の拡張の zip）を標準ライブラリだけで作る（S059）。

@vscode/vsce はライセンスが確かめられない署名用のバイナリ（@vscode/vsce-sign）を引き込むので使わない。
VSIX は OPC の zip で、`[Content_Types].xml`・`extension.vsixmanifest`・`extension/` の下の拡張の本体からなる。
使い方: `pnpm build && python3 -I pack_vsix.py` → `globe-mode-<版>.vsix`
"""

import json
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

HERE = Path(__file__).resolve().parent
# 拡張子の無いファイル（LICENSE-MIT など）は OPC で型が決まらないので .txt にして入れる
FILES = {
    "package.json": "package.json",
    "dist/extension.js": "dist/extension.js",
    "language-configuration.json": "language-configuration.json",
    "README.md": "README.md",
    "LICENSE-MIT": "LICENSE-MIT.txt",
    "LICENSE-APACHE": "LICENSE-APACHE.txt",
}

CONTENT_TYPES = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json"/>
  <Default Extension=".js" ContentType="application/javascript"/>
  <Default Extension=".md" ContentType="text/markdown"/>
  <Default Extension=".txt" ContentType="text/plain"/>
  <Default Extension=".vsixmanifest" ContentType="text/xml"/>
</Types>
"""


def manifest(pkg: dict) -> str:
    e = lambda s: escape(str(s), {'"': "&quot;"})  # noqa: E731
    return f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="{e(pkg['name'])}" Version="{e(pkg['version'])}" Publisher="{e(pkg['publisher'])}"/>
    <DisplayName>{e(pkg['displayName'])}</DisplayName>
    <Description xml:space="preserve">{e(pkg['description'])}</Description>
    <Categories>{e(','.join(pkg.get('categories', [])))}</Categories>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="{e(pkg['engines']['vscode'])}"/>
      <Property Id="Microsoft.VisualStudio.Code.ExtensionKind" Value="workspace"/>
    </Properties>
    <License>extension/LICENSE-MIT.txt</License>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/LICENSE-MIT.txt" Addressable="true"/>
  </Assets>
</PackageManifest>
"""


def main() -> int:
    pkg = json.loads((HERE / "package.json").read_text(encoding="utf-8"))
    missing = [s for s in FILES if not (HERE / s).is_file()]
    if missing:
        print(f"無いファイル: {missing}（先に pnpm build）", file=sys.stderr)
        return 1
    out = HERE / f"{pkg['name']}-{pkg['version']}.vsix"
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", CONTENT_TYPES)
        z.writestr("extension.vsixmanifest", manifest(pkg))
        for src, dst in FILES.items():
            z.write(HERE / src, f"extension/{dst}")
    print(out.name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
