# CI explicitly provisions the native JDK and Ant; Oyzu never downloads tools.
FROM eclipse-temurin:17.0.20.1_1-jdk-jammy
ADD https://downloads.apache.org/ant/binaries/apache-ant-1.10.18-bin.tar.gz /tmp/ant.tar.gz
RUN echo 'c510d744876d8da48dabc9495b023b6ec5284a0f18b40ee50d98ce099e791ca5c73e3cd4c80d3c90d3efc03f9620be43aad0a373ed47198284f7e3373e0db2c3  /tmp/ant.tar.gz' | sha512sum --check \
    && mkdir -p /opt/ant \
    && tar -xzf /tmp/ant.tar.gz -C /opt/ant --strip-components=1 \
    && rm /tmp/ant.tar.gz
ENV ANT_HOME=/opt/ant
ENV PATH=/opt/ant/bin:$PATH
