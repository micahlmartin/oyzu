# Wrapper preparation

This fixture uses an installed Gradle 8+ and JDK 17+. It deliberately contains no fabricated wrapper JAR or distribution checksum. Wrapper acquisition remains an unverified variant.

After choosing a pinned approved distribution, generate its wrapper with native Gradle and configure its authentic distributionSha256Sum. Preserve upstream license notices before checking generated third-party wrapper code into this repository. See the [official wrapper guide](https://docs.gradle.org/current/userguide/gradle_wrapper.html).
