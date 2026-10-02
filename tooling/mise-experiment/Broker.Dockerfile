FROM oyzu-mise-experiment:trace
COPY oyzu-mise-spike /usr/local/bin/oyzu-mise-spike
COPY UPSTREAM-LICENSE.txt dependency-licenses.json /usr/share/doc/oyzu-mise-spike/
COPY broker_bridge.py broker_worker.py broker_transport.py /opt/qualification/
RUN chmod +x /usr/local/bin/oyzu-mise-spike
ENV PYTHONDONTWRITEBYTECODE=1
WORKDIR /workspace
