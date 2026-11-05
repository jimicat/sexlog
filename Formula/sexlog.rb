class Sexlog < Formula
  desc "Local, private sex-life logging CLI with a static dashboard"
  homepage "https://github.com/jimicat/sexlog"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "5a5edcb81ca42626d729b9a7f73c165f222d153f52917f52915b18fc7d88098c"
    else
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "40c0fa6b7788c71e760795cf0be375e3a349d6e68cbdc70ee162b3364418a657"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "5b5b405d5a831d0f935329d5d21c9186e1021c7e71202210160f860035da63c5"
    else
      url "https://github.com/jimicat/sexlog/releases/download/v#{version}/sexlog-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "ac942634cdb808452396d69f3b8290653e1f390a0c91b9737539e884a86358ee"
    end
  end

  def install
    bin.install "sexlog"
  end

  test do
    assert_match "sexlog", shell_output("#{bin}/sexlog --version")
  end
end
