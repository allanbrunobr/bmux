# Formula/bmux.rb — Homebrew formula for BMUX
# Story 7.3: Homebrew Formula [v1.0]
#
# Tap: brew install bruno/tap/bmux
#
# To update for a new release:
#   1. Download the new macos-* binary
#   2. Run: shasum -a 256 bmux-macos-*
#   3. Update `url` and `sha256` for each resource block below
#   4. Update `version`

class Bmux < Formula
  desc "Terminal multiplexer with AI agent orchestration"
  homepage "https://github.com/brunobsc/bmux"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  # ─── macOS Apple Silicon ──────────────────────────────────────────────────
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/brunobsc/bmux/releases/download/v#{version}/bmux-macos-aarch64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "bmux-macos-aarch64" => "bmux"
        install_man_page
      end
    end

    # ─── macOS Intel ────────────────────────────────────────────────────────
    if Hardware::CPU.intel?
      url "https://github.com/brunobsc/bmux/releases/download/v#{version}/bmux-macos-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "bmux-macos-x86_64" => "bmux"
        install_man_page
      end
    end
  end

  # ─── Linux x86_64 ────────────────────────────────────────────────────────
  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/brunobsc/bmux/releases/download/v#{version}/bmux-linux-x86_64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "bmux-linux-x86_64" => "bmux"
        install_man_page
      end
    end

    # ─── Linux aarch64 ──────────────────────────────────────────────────────
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/brunobsc/bmux/releases/download/v#{version}/bmux-linux-aarch64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "bmux-linux-aarch64" => "bmux"
        install_man_page
      end
    end
  end

  # ─── Shared helpers ──────────────────────────────────────────────────────
  def install_man_page
    # Man page is fetched from the release source archive
    man1.install "bmux.1"
  end

  # ─── Tests ───────────────────────────────────────────────────────────────
  test do
    # Verify the binary runs and reports its version
    output = shell_output("#{bin}/bmux --version")
    assert_match version.to_s, output

    # Verify subcommands are listed
    help_output = shell_output("#{bin}/bmux --help")
    %w[new attach ls kill agent task context workflow config data].each do |cmd|
      assert_match cmd, help_output
    end
  end
end
