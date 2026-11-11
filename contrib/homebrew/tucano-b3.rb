# IMPORTANT: Update sha256 values after each release.
# Get values from: https://github.com/tucano-b3/tucano-b3/releases

class TucanoB3 < Formula
  desc "API REST de alta performance para dados de mercado da B3"
  homepage "https://github.com/tucano-b3/tucano-b3"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tucano-b3/tucano-b3/releases/download/v0.1.0/tucano-b3-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    else
      url "https://github.com/tucano-b3/tucano-b3/releases/download/v0.1.0/tucano-b3-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tucano-b3/tucano-b3/releases/download/v0.1.0/tucano-b3-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    else
      url "https://github.com/tucano-b3/tucano-b3/releases/download/v0.1.0/tucano-b3-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  def install
    bin.install "tucano-b3"
  end

  test do
    assert_match "tucano-b3", shell_output("#{bin}/tucano-b3 --help")
  end
end

