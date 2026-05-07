class Ctx < Formula
  desc "Local-first context runtime engine for coding agents"
  homepage "https://github.com/Alegau03/CTX"
  url "https://github.com/Alegau03/CTX/archive/refs/tags/v0.2.5.tar.gz"
  sha256 "8916c94d701a91d499cdea1dc4bd5a780e5528ed93984f844bd56daa1d588960"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", "crates/ctx-cli", "--root", prefix
  end

  test do
    system bin/"ctx", "help"
    test_repo = testpath/"demo"
    test_repo.mkpath
    system bin/"ctx", "--repo-root", test_repo, "doctor"
    system bin/"ctx", "--repo-root", test_repo, "init"
    output = shell_output("#{bin}/ctx --repo-root #{test_repo} doctor")
    assert_match "config: ok", output
    assert_match "local_only: true", output
  end
end
