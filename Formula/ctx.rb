class Ctx < Formula
  desc "Local-first context runtime engine for coding agents"
  homepage "https://github.com/Alegau03/CTX"
  url "https://github.com/Alegau03/CTX/archive/refs/tags/v0.2.4.tar.gz"
  sha256 "7865c569d0745c9f7c1182723f859f483be7eadedef6f17f7217c7b80ddf0eec"
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
