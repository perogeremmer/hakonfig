// Cetak formula Homebrew untuk hakonfig-cli ke stdout.
//
// Dipakai oleh .github/workflows/homebrew-tap.yml setelah sebuah release
// di-publish. Formula menunjuk aset binary di GitHub release dan mengunci
// sha256-nya, jadi dijalankan hanya setelah aset benar-benar ada:
//   node scripts/gen-homebrew-formula.mjs v0.4.0 > Formula/hakonfig-cli.rb
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";

const REPO = "perogeremmer/hakonfig";
const tag = process.argv[2];
if (!tag) {
  console.error("pakai: node scripts/gen-homebrew-formula.mjs <tag>");
  process.exit(1);
}

const version = tag.replace(/^v/, "");

/** { name, url } aset CLI dari release tertentu. */
const release = JSON.parse(
  execFileSync("gh", ["api", `repos/${REPO}/releases/tags/${tag}`], {
    encoding: "utf8",
  })
);
const assets = Object.fromEntries(
  release.assets.map((asset) => [asset.name, asset.browser_download_url])
);

const WANTED = {
  darwin: "hakonfig-cli-universal-apple-darwin",
  linux: "hakonfig-cli-x86_64-unknown-linux-gnu",
};
for (const name of Object.values(WANTED)) {
  if (!assets[name]) {
    console.error(`aset '${name}' tidak ada di release ${tag}`);
    process.exit(1);
  }
}

/** Unduh aset dan kembalikan sha256-nya. */
function sha256(name) {
  const dir = mkdtempSync(join(tmpdir(), "hakonfig-formula-"));
  execFileSync("curl", ["-fsSL", assets[name], "-o", join(dir, name)]);
  return createHash("sha256")
    .update(readFileSync(join(dir, name)))
    .digest("hex");
}

const darwinSha = sha256(WANTED.darwin);
const linuxSha = sha256(WANTED.linux);

process.stdout.write(`class HakonfigCli < Formula
  desc "Manage AI coding harness provider configs"
  homepage "https://github.com/${REPO}"
  version "${version}"
  license "MIT"

  head "https://github.com/${REPO}.git", branch: "master"

  on_macos do
    url "${assets[WANTED.darwin]}"
    sha256 "${darwinSha}"

    def install
      binary = "hakonfig-cli"
      bin.install "${WANTED.darwin}" => binary
    end
  end

  on_linux do
    url "${assets[WANTED.linux]}"
    sha256 "${linuxSha}"

    def install
      binary = "hakonfig-cli"
      bin.install "${WANTED.linux}" => binary
    end
  end

  def caveats
    <<~EOS
      The CLI is installed as hakonfig-cli.
      Desktop app docs: https://github.com/${REPO}#readme
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hakonfig-cli --version")
  end
end
`);
