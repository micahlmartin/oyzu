# CI explicitly provisions the native JDK and Ant; Oyzu never downloads tools.
FROM eclipse-temurin:17.0.20.1_1-jdk-jammy
RUN apt-get update && apt-get install -y --no-install-recommends python3 && rm -rf /var/lib/apt/lists/*
ADD https://downloads.apache.org/ant/binaries/apache-ant-1.10.18-bin.tar.gz /tmp/ant.tar.gz
RUN echo 'c510d744876d8da48dabc9495b023b6ec5284a0f18b40ee50d98ce099e791ca5c73e3cd4c80d3c90d3efc03f9620be43aad0a373ed47198284f7e3373e0db2c3  /tmp/ant.tar.gz' | sha512sum --check \
    && mkdir -p /opt/ant \
    && tar -xzf /tmp/ant.tar.gz -C /opt/ant --strip-components=1 \
    && rm /tmp/ant.tar.gz
ENV ANT_HOME=/opt/ant
ENV PATH=/opt/ant/bin:$PATH
ADD https://repo.maven.apache.org/maven2/org/jacoco/org.jacoco.ant/0.8.13/org.jacoco.ant-0.8.13-nodeps.jar /opt/jacoco/jacocoant.jar
ADD https://repo.maven.apache.org/maven2/org/jacoco/org.jacoco.agent/0.8.13/org.jacoco.agent-0.8.13-runtime.jar /opt/jacoco/jacocoagent.jar
RUN echo 'a1705e3b1e14a86e7fe390192014af42076b730256130b4be40720e36ddd3f32  /opt/jacoco/jacocoant.jar' | sha256sum --check \
    && echo '47e700ccb0fdb9e27c5241353f8161938f4e53c3561dd35e063c5fe88dc3349b  /opt/jacoco/jacocoagent.jar' | sha256sum --check \
    && chmod 644 /opt/jacoco/*.jar
ADD https://repo.maven.apache.org/maven2/junit/junit/4.13.2/junit-4.13.2.jar /opt/ant/lib/junit-4.13.2.jar
ADD https://repo.maven.apache.org/maven2/org/hamcrest/hamcrest-core/1.3/hamcrest-core-1.3.jar /opt/ant/lib/hamcrest-core-1.3.jar
RUN echo '8e495b634469d64fb8acfa3495a065cbacc8a0fff55ce1e31007be4c16dc57d3  /opt/ant/lib/junit-4.13.2.jar' | sha256sum --check \
    && echo '66fdef91e9739348df7a096aa384a5685f4e875584cce89386a7a47251c4d8e9  /opt/ant/lib/hamcrest-core-1.3.jar' | sha256sum --check \
    && chmod 644 /opt/ant/lib/*.jar

COPY tooling/provision-java-quality.py /tmp/oyzu-provision/tooling/provision-java-quality.py
COPY tooling/provisioning.py /tmp/oyzu-provision/tooling/provisioning.py
COPY src/builders/java/runtime /tmp/oyzu-provision/src/builders/java/runtime
RUN python3 /tmp/oyzu-provision/tooling/provision-java-quality.py --destination /opt/oyzu-java-quality \
    && chmod -R a+rX /opt/oyzu-java-quality \
    && rm -rf /tmp/oyzu-provision
ENV OYZU_JAVA_QUALITY_HOME=/opt/oyzu-java-quality
