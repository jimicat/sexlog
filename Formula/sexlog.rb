class Sexlog < Formula
  desc "Local, private sex-life logging CLI with a static dashboard"
  homepage "https://github.com/jimicat/sexlog"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "894d9ba9904ca3d4fbb5faba034ec075576b06d3d460b377c254053c8cbec77f"
    else
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "1299e88dc8216d7fee99ac1f2a7bd54b03ec10521d9dd5eaaedadf081dcd3d4b"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "229845813513a1c851f429d3034ed14d0519d7b55067a3ff12f6e666d3979795"
    else
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a730a40a364bb519da4a77620b19232db6afc9ad297f6df445a400e3b0f0ccea"
    end
  end

  def install
    bin.install "sexlog"
  end

  test do
    assert_match "sexlog", shell_output("#{bin}/sexlog --version")
  end
end
