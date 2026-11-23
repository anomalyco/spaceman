// The release the site offers. The page and its JSON-LD (vite.config.ts) read from here; public/llms.txt
// and public/llms-full.txt repeat the version and download by hand.
export const RELEASE_VERSION = "0.3.0";
export const MIN_MACOS_LABEL = "macOS 14 or later";
const RELEASES = "https://pub-6f85d683ffe0419cad0a71508904053a.r2.dev/releases";
export const DOWNLOAD_URL = `${RELEASES}/Spaceman-v${RELEASE_VERSION}-macos-arm64.zip`;
export const CHECKSUM_URL = `${DOWNLOAD_URL}.sha256`;
export const SITE_URL = "https://anoma.ly/spaceman/";
export const INSTALL = `curl -fL ${RELEASES}/Spaceman-latest-macos-arm64.zip \\
  -o /tmp/Spaceman.zip && ditto -x -k /tmp/Spaceman.zip /Applications \\
  && open /Applications/Spaceman.app`;
