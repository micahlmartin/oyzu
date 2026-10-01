# CI provisions this toolchain explicitly; Oyzu does not install tools.
FROM eclipse-temurin:17.0.20.1_1-jdk-jammy
RUN apt-get update && apt-get install -y --no-install-recommends python3 unzip \
    && rm -rf /var/lib/apt/lists/*
ADD https://downloads.gradle.org/distributions/gradle-8.14.3-bin.zip /tmp/gradle.zip
RUN echo 'bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531  /tmp/gradle.zip' | sha256sum --check \
    && unzip -q /tmp/gradle.zip -d /opt \
    && rm /tmp/gradle.zip
ENV GRADLE_HOME=/opt/gradle-8.14.3
ENV PATH=/opt/gradle-8.14.3/bin:$PATH
