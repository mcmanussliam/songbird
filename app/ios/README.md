This directory is intentionally not hand-authored. Once Flutter SDK is available, run:

    flutter create --platforms=ios .

from `app/` to generate the real iOS project, then wire the songbird-core static library into
the Xcode project per flutter_rust_bridge's iOS integration docs (M3).
