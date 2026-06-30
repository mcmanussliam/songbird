This directory is intentionally not hand-authored. Once Flutter SDK is available, run:

    flutter create --platforms=android .

from `app/` to generate the real Android project, then wire the songbird-core shared library
into the Gradle build per flutter_rust_bridge's Android integration docs (M3). Application ID:
`org.songbird.app`.
