class Robocopy2rust < Formula
  desc "Pure-Rust Iced GUI, CLI job builder, .rcj/script exporter, and MCP server for Microsoft Robocopy"
  homepage "https://robocopy2rust.guap.ca"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/adpaolucci/robocopy2rust/releases/download/v0.1.0/robocopy2rust-aarch64-apple-darwin.tar.gz"
      sha256 "b0505479aac88c546f0b2b74d2f75e209fbe7734c18204d843c96214b823c17c"
    else
      url "https://github.com/adpaolucci/robocopy2rust/releases/download/v0.1.0/robocopy2rust-x86_64-apple-darwin.tar.gz"
      sha256 "11a236c782668459f62e9f2a6b21e0acab4b92e8926cbf201212c496acd2eef0"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/adpaolucci/robocopy2rust/releases/download/v0.1.0/robocopy2rust-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "3db503180dc1b50cc7544986303a24c56a9e3398f06a9b70c6f4c3bc195f1718"
    end
  end

  def install
    bin.install "robocopy2rust"
    bin.install "robocopy2rust-iced"
    bin.install "robocopy2rust-mcp"
    doc.install "README.md", "CHANGELOG.md"
    doc.install "SBOM-crates.txt" if File.exist?("SBOM-crates.txt")
    prefix.install "LICENSE", "NOTICE"
  end

  test do
    assert_match "robocopy2rust", shell_output("#{bin}/robocopy2rust --help")
    assert_match "0x01", shell_output("#{bin}/robocopy2rust decode 1")
    assert_match "robocopy2rust-iced", shell_output("#{bin}/robocopy2rust-iced --help")
  end
end
