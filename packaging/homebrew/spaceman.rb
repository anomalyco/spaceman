cask "spaceman" do
  version "@VERSION@"
  sha256 "@SHA256@"

  url "@URL@"
  name "Spaceman"
  desc "Tiling window manager with stable logical windows"
  homepage "https://anoma.ly/spaceman/"

  livecheck do
    url "https://anoma.ly/release/spaceman/desktop/stable.json"
    strategy :json do |json|
      json.dig("release", "version")
    end
  end

  auto_updates true
  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Spaceman.app"
  binary "#{appdir}/Spaceman.app/Contents/MacOS/spaceman"

  uninstall quit: "anomaly.spaceman"

  zap trash: [
    "~/.config/spaceman",
    "~/Library/Application Support/Spaceman",
    "~/Library/Caches/anomaly.spaceman",
    "~/Library/HTTPStorages/anomaly.spaceman",
    "~/Library/Preferences/anomaly.spaceman.plist",
    "~/Library/Saved Application State/anomaly.spaceman.savedState",
  ]
end
