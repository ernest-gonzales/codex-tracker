cask "codex-tracker" do
  version "0.1.0"

  on_arm do
    sha256 "REPLACE_WITH_ARM64_SHA256"
    url "https://github.com/<org>/codex-tracker/releases/download/v#{version}/codex-tracker_#{version}_arm64.dmg"
  end

  on_intel do
    sha256 "REPLACE_WITH_X86_64_SHA256"
    url "https://github.com/<org>/codex-tracker/releases/download/v#{version}/codex-tracker_#{version}_x86_64.dmg"
  end

  name "Codex Tracker"
  desc "Local-only Codex CLI usage tracker"
  homepage "https://github.com/<org>/codex-tracker"

  app "Codex Tracker.app"
end
