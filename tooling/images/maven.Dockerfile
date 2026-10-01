# CI provisions this toolchain explicitly; Oyzu does not install tools.
FROM eclipse-temurin:17.0.20.1_1-jdk-jammy
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
ADD https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.11/apache-maven-3.9.11-bin.tar.gz /tmp/maven.tar.gz
RUN echo 'bcfe4fe305c962ace56ac7b5fc7a08b87d5abd8b7e89027ab251069faebee516b0ded8961445d6d91ec1985dfe30f8153268843c89aa392733d1a3ec956c9978  /tmp/maven.tar.gz' | sha512sum --check \
    && mkdir -p /opt/maven /opt/oyzu-maven/classes/META-INF/plexus \
    && tar -xzf /tmp/maven.tar.gz -C /opt/maven --strip-components=1 \
    && rm /tmp/maven.tar.gz
COPY src/builders/java/maven/runtime/OyzuMetadata.java /opt/oyzu-maven/OyzuMetadata.java
COPY src/builders/java/maven/runtime/components.xml /opt/oyzu-maven/classes/META-INF/plexus/components.xml
RUN javac --release 17 -cp '/opt/maven/lib/*' -d /opt/oyzu-maven/classes /opt/oyzu-maven/OyzuMetadata.java \
    && jar --create --file /opt/oyzu-maven/metadata.jar --date=1980-01-01T00:00:02Z -C /opt/oyzu-maven/classes .
ENV MAVEN_HOME=/opt/maven
ENV PATH=/opt/maven/bin:$PATH
